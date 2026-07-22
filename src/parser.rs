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
    Attr, Card, Class, ClassDiagram, ClassRel, Direction, Document, EdgeKind, End, ErDiagram,
    FrameKind, Graph, Journey, JourneySection, JourneyTask, Key, Member, MindNode, MindShape,
    Mindmap, NodeStyle, NoteSide, PieChart, PieSlice, RelKind, Relation, SeqHead, SeqItem,
    SequenceDiagram, Shape, SubEdge, Subgraph, Visibility,
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

/// Hard cap on nesting depth — subgraph blocks and mindmap branches.
/// Real diagrams nest a handful of levels; this exists only to turn a
/// pathological input (thousands of levels) into a clean, line-numbered
/// parse error instead of a stack overflow: the layout pass recurses one
/// frame per level, so unbounded nesting would abort the process. Set
/// well above any sane diagram yet far below the native stack limit —
/// and low enough to stay safe on the smaller wasm stack.
const MAX_NEST_DEPTH: usize = 500;

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
/// (`A["odd text"]`), turn `<br>` / `<br/>` / `<br />` line breaks
/// into real newlines (mermaid renders those as line breaks), and
/// decode mermaid entity codes (`#quot;`, `#35;`, …).
fn clean_label(s: &str) -> String {
    let t = s.trim();
    let inner = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        t
    };
    // Trim leading/trailing blank lines from `<br/>` at the edges
    // (`"<br/>"` → "" not two empty lines).
    decode_entities(&normalize_breaks(inner))
        .trim_matches('\n')
        .to_string()
}

/// Decode mermaid's escape entities: `#quot;` → `"` and numeric
/// `#NN;` → the character with that decimal code point. This is how
/// mermaid smuggles characters its grammar reserves (`"`, `|`, `#`)
/// into labels — and how [`crate::emit::to_mermaid`] writes them
/// back out losslessly. Anything not matching the pattern passes
/// through unchanged (`#f00`, `C#`, a lone `#`).
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(h) = rest.find('#') {
        out.push_str(&rest[..h]);
        let tail = &rest[h + 1..];
        if let Some(t) = tail.strip_prefix("quot;") {
            out.push('"');
            rest = t;
            continue;
        }
        let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && tail[digits.len()..].starts_with(';') {
            // Refuse to manufacture control characters (except the
            // whitespace trio): `#0;` / `#27;` would poison labels —
            // and the emitted SVG — with bytes XML can't carry.
            let ok = digits
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .filter(|c| matches!(c, '\t' | '\n') || !c.is_control());
            if let Some(c) = ok {
                out.push(c);
                rest = &tail[digits.len() + 1..];
                continue;
            }
        }
        out.push('#');
        rest = tail;
    }
    out.push_str(rest);
    out
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
    // Editor Windows (Notepad, PowerShell `>`) menyisipkan BOM UTF-8.
    // U+FEFF bukan whitespace Unicode, jadi lolos trim() dan membuat
    // header tak terdeteksi dengan error yang menyesatkan.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        return match diagram_type(line) {
            Some("erDiagram") => parse_er(source, i + 1).map(Document::Er),
            Some("classDiagram") => parse_class(source, i + 1).map(Document::Class),
            Some("sequenceDiagram") => parse_sequence(source, i + 1).map(Document::Sequence),
            Some("pie") => parse_pie(source, i + 1).map(Document::Pie),
            Some("stateDiagram-v2") | Some("stateDiagram") => {
                parse_state(source, i + 1).map(Document::State)
            }
            Some("mindmap") => parse_mindmap(source, i + 1).map(Document::Mindmap),
            Some("journey") => parse_journey(source, i + 1).map(Document::Journey),
            Some(t) => Err(err(
                i + 1,
                format!(
                    "diagram type '{}' is not supported yet (supported: flowchart, \
                     graph, erDiagram, classDiagram, sequenceDiagram, pie, \
                     stateDiagram-v2, mindmap, journey)",
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
    // Lihat catatan BOM di parse_document — entry point ini publik
    // juga, jadi dapat perlakuan yang sama.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
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
    // order matches the index each block gets during the main pass —
    // so the pre-scan and the main loop MUST classify every
    // `subgraph …` line identically, or sub-edges bind to the wrong
    // box. `subgraph --> B` is only ever an edge (not a header) when
    // a subgraph literally NAMED "subgraph" exists; that existence
    // check is decidable up front because such a header line can't
    // itself look like an edge.
    let has_sg_named_subgraph = source.lines().any(|raw| {
        strip_keyword(raw.trim(), "subgraph").is_some_and(|rest| {
            let r = rest.trim();
            r == "subgraph" || r.starts_with("subgraph[") || r.starts_with("subgraph [")
        })
    });
    let sg_line_is_edge = |rest: &str| edge_op_follows(rest) && has_sg_named_subgraph;
    let mut sub_ids: HashMap<String, usize> = HashMap::new();
    {
        let mut idx = 0usize;
        for raw in source.lines() {
            if let Some(rest) = strip_keyword(raw.trim(), "subgraph") {
                if sg_line_is_edge(rest) {
                    continue;
                }
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
                let hint = if matches!(
                    t,
                    "erDiagram"
                        | "classDiagram"
                        | "sequenceDiagram"
                        | "pie"
                        | "stateDiagram-v2"
                        | "stateDiagram"
                ) {
                    "this parser is flowchart-only — use parse_document() or render_svg()"
                } else {
                    "not supported yet (supported: flowchart, graph, erDiagram, \
                     classDiagram, sequenceDiagram, pie, stateDiagram-v2, mindmap, journey)"
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

        // A statement keyword followed by an edge operator (or a
        // fan-out `&`) is not that statement — it's an edge whose
        // FROM side is a subgraph NAMED like the keyword (`subgraph
        // style` … `style --> B`). The divert is gated on the
        // pre-scanned subgraph ids ONLY: that set is complete before
        // this loop runs, so the decision can't depend on line order
        // (nodes-so-far would), and a stray `style --> B` with no
        // such subgraph keeps its 0.19-era statement diagnostic.
        // (Keyword-named NODES never need this path: the emitter
        // always writes them in full `id["label"]` form.)
        let names_a_subgraph = |kw: &str| sub_ids.contains_key(kw);

        // Subgraph blocks: `subgraph id [Title]` ... `end`, nestable,
        // with an optional `direction XX` line inside.
        if let Some(rest) = strip_keyword(line, "subgraph") {
            if sg_line_is_edge(rest) {
                parse_statement(&mut g, line, lineno, &mut assigns, &sub_stack, &sub_ids)?;
                continue;
            }
            if sub_stack.len() >= MAX_NEST_DEPTH {
                return Err(err(
                    lineno,
                    format!("subgraph nesting too deep (max {MAX_NEST_DEPTH} levels)"),
                ));
            }
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
            if !(edge_op_follows(rest) && names_a_subgraph("direction")) {
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
        }

        // Styling statements (`style A fill:...`, `classDef name ...`,
        // `class A,B name`). strip_keyword requires whitespace after
        // the keyword, so `class` can't shadow `classDef`.
        if let Some(rest) = strip_keyword(line, "classDef") {
            if edge_op_follows(rest) && names_a_subgraph("classDef") {
                parse_statement(&mut g, line, lineno, &mut assigns, &sub_stack, &sub_ids)?;
                continue;
            }
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
            if edge_op_follows(rest) && names_a_subgraph("class") {
                parse_statement(&mut g, line, lineno, &mut assigns, &sub_stack, &sub_ids)?;
                continue;
            }
            let rest = rest.trim();
            let (ids, name) = rest.split_once(char::is_whitespace).ok_or_else(|| {
                err(lineno, "class needs node ids and a class name".to_string())
            })?;
            for id in ids.split(',').filter(|i| !i.is_empty()) {
                let n = g.ensure_node(check_stmt_id(id.trim(), "class", lineno)?, None, None);
                assigns.push((n, name.trim().to_string()));
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "style") {
            if edge_op_follows(rest) && names_a_subgraph("style") {
                parse_statement(&mut g, line, lineno, &mut assigns, &sub_stack, &sub_ids)?;
                continue;
            }
            let rest = rest.trim();
            let (id, props) = rest.split_once(char::is_whitespace).ok_or_else(|| {
                err(lineno, "style needs a node id and properties".to_string())
            })?;
            let n = g.ensure_node(check_stmt_id(id.trim(), "style", lineno)?, None, None);
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
    // Split on commas at paren depth 0 only, so CSS function values
    // (`fill:rgb(255,0,0)`) survive as one property. An unbalanced
    // '(' would otherwise swallow every later property silently —
    // reject it with a proper error instead.
    let mut items: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(err(lineno, format!("unbalanced '(' in style properties: '{s}'")));
    }
    items.push(&s[start..]);
    for item in items {
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

/// Whether `rest` (the text after a statement keyword) begins with a
/// COMPLETE edge operator or a fan-out `&` — i.e. the "keyword" is
/// really a node/subgraph name starting an edge (`style --> B`,
/// `style & A --> B`), not a statement. Uses the real operator
/// recognizer so `--x` (a legal statement argument in old files)
/// does NOT count as an edge.
fn edge_op_follows(rest: &str) -> bool {
    let r = rest.trim_start();
    r.starts_with('&') || parse_edge_op(&mut Cur::new(r)).is_some()
}

/// A `class`/`style` statement id must be word-like — an id such as
/// `-->` (from a mangled edge line landing in the statement branch)
/// would otherwise become a silent phantom node that no mermaid text
/// can ever express again.
fn check_stmt_id<'a>(id: &'a str, stmt: &str, lineno: usize) -> Result<&'a str, ParseError> {
    if !id.is_empty() && id.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Ok(id)
    } else {
        Err(err(
            lineno,
            format!("{stmt} needs a node id, found: '{id}'"),
        ))
    }
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

// ---------------------------------------------------------------
// Class diagram (`classDiagram`)
// ---------------------------------------------------------------

/// Parse a `classDiagram` body. `header_line` is 1-indexed.
fn parse_class(source: &str, header_line: usize) -> Result<ClassDiagram, ParseError> {
    let mut d = ClassDiagram::default();
    // (class index, line where its `{` opened) for a good diagnostic.
    let mut open: Option<(usize, usize)> = None;
    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        // Strip a trailing `%% comment` (kept out of quotes) so a
        // comment after a relation/member doesn't corrupt the parse.
        let line = match find_unquoted(line, "%%") {
            Some(p) => line[..p].trim(),
            None => line,
        };
        if line.is_empty() {
            continue;
        }
        if let Some((ci, _)) = open {
            if line == "}" {
                open = None;
            } else if !line.starts_with("<<") {
                // Skip `<<interface>>` / `<<abstract>>` stereotypes
                // (not yet rendered) rather than treat them as fields.
                add_class_member(&mut d.classes[ci], line);
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "class") {
            let rest = rest.trim();
            // `class Name {` opens a block; `class Name` just declares.
            if let Some(name) = rest.strip_suffix('{') {
                let ci = d.ensure_class(name.trim());
                open = Some((ci, lineno));
            } else {
                // `class A, B, C` declares several at once (Mermaid).
                for name in rest.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    d.ensure_class(name);
                }
            }
            continue;
        }
        // Ignore directives we don't render yet (`direction LR`,
        // `note ...`) instead of mis-reading them as relations.
        if strip_keyword(line, "direction").is_some() || strip_keyword(line, "note").is_some() {
            continue;
        }
        // Classify by the part BEFORE the first unquoted colon: if it
        // holds no relation operator, this is an inline member
        // `Name : member` — quoted cardinalities and member text must
        // not sway the decision (`Animal : has -- dashes` is a member,
        // `A "1:n" --> B` is a relation).
        let colon = find_unquoted(line, ":");
        let head = colon.map_or(line, |c| line[..c].trim());
        if find_class_op(head).is_none() {
            if let Some(c) = colon {
                let name = line[..c].trim();
                let member = line[c + 1..].trim();
                if is_class_ident(name) && !member.is_empty() {
                    let ci = d.ensure_class(name);
                    add_class_member(&mut d.classes[ci], member);
                    continue;
                }
            }
        }
        parse_class_relation(&mut d, line, lineno)?;
    }
    if let Some((ci, oln)) = open {
        return Err(err(
            oln,
            format!("class block '{}' is never closed with '}}'", d.classes[ci].name),
        ));
    }
    Ok(d)
}

fn is_class_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// First byte index of `needle` in `s` that lies outside any `"..."`
/// quoted region, so a colon or operator inside a quoted cardinality
/// (`"1:n"`, `"*--"`) is ignored. `needle` is expected to be ASCII.
fn find_unquoted(s: &str, needle: &str) -> Option<usize> {
    let mut in_quote = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
        } else if !in_quote && s[i..].starts_with(needle) {
            return Some(i);
        }
    }
    None
}

/// Locate the relation operator in `rel`: the earliest position
/// outside any quoted cardinality where a [`CLASS_OPS`] operator
/// matches (longest wins, since the table is ordered longest-first).
/// Returns `(byte position, op, kind, dashed, head_at_left)`.
///
/// `o` is a valid identifier char, so an `o--` / `--o` marker only
/// counts at an operand boundary — never inside a name like `Foo`.
fn find_class_op(rel: &str) -> Option<(usize, &'static str, RelKind, bool, bool)> {
    let mut in_quote = false;
    let mut prev: Option<char> = None;
    for (i, c) in rel.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
        } else if !in_quote {
            if let Some(&(op, k, dash, hl)) = CLASS_OPS.iter().find(|t| rel[i..].starts_with(t.0)) {
                let before_ok = !op.starts_with('o') || prev.map_or(true, |p| !p.is_alphanumeric());
                let after_ok = !op.ends_with('o')
                    || !rel[i + op.len()..].starts_with(|c: char| c.is_alphanumeric());
                if before_ok && after_ok {
                    return Some((i, op, k, dash, hl));
                }
            }
        }
        prev = Some(c);
    }
    None
}

/// Add one field or method from a member line like `+name: Type` or
/// `+doWork(x) Ret`. A `()` marks a method.
fn add_class_member(c: &mut Class, s: &str) {
    let s = s.trim();
    let (vis, rest) = match s.chars().next() {
        Some('+') => (Visibility::Public, &s[1..]),
        Some('-') => (Visibility::Private, &s[1..]),
        Some('#') => (Visibility::Protected, &s[1..]),
        Some('~') => (Visibility::Package, &s[1..]),
        _ => (Visibility::None, s),
    };
    let m = Member {
        visibility: vis,
        text: rest.trim().to_string(),
    };
    if rest.contains('(') {
        c.methods.push(m);
    } else {
        c.fields.push(m);
    }
}

/// Relation operators, longest first. Each maps to
/// (kind, dashed, head_at_left).
const CLASS_OPS: &[(&str, RelKind, bool, bool)] = &[
    ("<|--", RelKind::Inheritance, false, true),
    ("--|>", RelKind::Inheritance, false, false),
    ("<|..", RelKind::Realization, true, true),
    ("..|>", RelKind::Realization, true, false),
    ("*--", RelKind::Composition, false, true),
    ("--*", RelKind::Composition, false, false),
    ("o--", RelKind::Aggregation, false, true),
    ("--o", RelKind::Aggregation, false, false),
    ("-->", RelKind::Association, false, false),
    ("<--", RelKind::Association, false, true),
    ("..>", RelKind::Dependency, true, false),
    ("<..", RelKind::Dependency, true, true),
    ("--", RelKind::Link, false, false),
    ("..", RelKind::Link, true, false),
];

/// `ClassA "card" <op> "card" ClassB : label`.
fn parse_class_relation(d: &mut ClassDiagram, line: &str, lineno: usize) -> Result<(), ParseError> {
    // Split off a trailing `: label` at the first colon OUTSIDE any
    // quoted cardinality (`"1:n"` keeps its colon).
    let (rel, label) = match find_unquoted(line, ":") {
        Some(c) => (line[..c].trim(), Some(clean_label_1line(line[c + 1..].trim()))),
        None => (line, None),
    };
    // Locate the operator: earliest position outside quotes, longest
    // match, honouring operand boundaries.
    let (opos, op, kind, dashed, head_left) = find_class_op(rel)
        .ok_or_else(|| err(lineno, format!("unknown class relation near: '{}'", rel)))?;
    let left = rel[..opos].trim();
    let right = rel[opos + op.len()..].trim();
    // Strip optional cardinality quotes from the inner edges.
    let (left_name, left_card) = split_card_end(left, true);
    let (right_name, right_card) = split_card_end(right, false);
    if !is_class_ident(left_name) || !is_class_ident(right_name) {
        return Err(err(
            lineno,
            format!("class relation needs two class names, got '{left_name}' / '{right_name}'"),
        ));
    }
    let li = d.ensure_class(left_name);
    let ri = d.ensure_class(right_name);
    // Normalise so the decorated end is `to`.
    let (from, to, from_card, to_card) = if head_left {
        (ri, li, right_card, left_card)
    } else {
        (li, ri, left_card, right_card)
    };
    d.relations.push(ClassRel {
        from,
        to,
        kind,
        dashed,
        from_card,
        to_card,
        label,
    });
    Ok(())
}

/// Split a `Name "card"` (card on the operator side) into
/// (name, card). `card_after` = the quote sits after the name
/// (left operand); otherwise before (right operand).
fn split_card_end(s: &str, card_after: bool) -> (&str, Option<String>) {
    let s = s.trim();
    if let (Some(q0), Some(q1)) = (s.find('"'), s.rfind('"')) {
        if q1 > q0 {
            let card = s[q0 + 1..q1].to_string();
            let name = if card_after {
                s[..q0].trim()
            } else {
                s[q1 + 1..].trim()
            };
            return (name, Some(card));
        }
    }
    (s, None)
}

// ---------------------------------------------------------------
// Sequence diagram (`sequenceDiagram`)
// ---------------------------------------------------------------

/// Parse a `sequenceDiagram` body. `header_line` is the 1-indexed
/// line of the header; everything up to and including it is skipped.
fn parse_sequence(source: &str, header_line: usize) -> Result<SequenceDiagram, ParseError> {
    let mut d = SequenceDiagram::default();
    // Open frames (kind, opening line) for `else`/`and`/`end`
    // matching and a good unclosed-frame diagnostic.
    let mut frames: Vec<(FrameKind, usize)> = Vec::new();
    // Activation depth per participant, for balance checking.
    let mut depth: Vec<usize> = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        // Strip a trailing `%% comment` (kept out of quotes) so a
        // comment after a message doesn't leak into its text.
        let line = match find_unquoted(line, "%%") {
            Some(p) => line[..p].trim(),
            None => line,
        };
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = strip_keyword(line, "participant") {
            parse_participant(&mut d, rest.trim(), false, lineno)?;
            continue;
        }
        if let Some(rest) = strip_keyword(line, "actor") {
            parse_participant(&mut d, rest.trim(), true, lineno)?;
            continue;
        }
        if let Some(rest) = strip_keyword(line, "autonumber") {
            if !rest.trim().is_empty() {
                return Err(err(
                    lineno,
                    format!("autonumber arguments are not supported yet: '{}'", rest.trim()),
                ));
            }
            d.autonumber = true;
            continue;
        }
        if let Some(rest) = strip_keyword(line, "note") {
            let item = parse_seq_note(&mut d, rest.trim(), lineno)?;
            d.items.push(item);
            continue;
        }
        if let Some(rest) = strip_keyword(line, "activate") {
            let p = seq_participant(&mut d, rest.trim(), lineno)?;
            grow(&mut depth, p);
            depth[p] += 1;
            d.items.push(SeqItem::Activate(p));
            continue;
        }
        if let Some(rest) = strip_keyword(line, "deactivate") {
            let p = seq_participant(&mut d, rest.trim(), lineno)?;
            grow(&mut depth, p);
            if depth[p] == 0 {
                return Err(err(
                    lineno,
                    format!(
                        "deactivate of '{}' without a matching activate",
                        d.participants[p].id
                    ),
                ));
            }
            depth[p] -= 1;
            d.items.push(SeqItem::Deactivate(p));
            continue;
        }
        // Frames: `loop|opt|alt|par <label>` … `else|and <label>` … `end`.
        const FRAMES: &[(&str, FrameKind)] = &[
            ("loop", FrameKind::Loop),
            ("opt", FrameKind::Opt),
            ("alt", FrameKind::Alt),
            ("par", FrameKind::Par),
        ];
        if let Some((kind, rest)) = FRAMES
            .iter()
            .find_map(|&(kw, k)| strip_keyword(line, kw).map(|r| (k, r)))
        {
            frames.push((kind, lineno));
            d.items.push(SeqItem::FrameStart {
                kind,
                label: clean_label_1line(rest.trim()),
            });
            continue;
        }
        if let Some(rest) = strip_keyword(line, "else") {
            if !matches!(frames.last(), Some((FrameKind::Alt, _))) {
                return Err(err(
                    lineno,
                    "'else' is only valid inside an open 'alt' frame".to_string(),
                ));
            }
            d.items.push(SeqItem::FrameElse {
                label: clean_label_1line(rest.trim()),
            });
            continue;
        }
        if let Some(rest) = strip_keyword(line, "and") {
            if !matches!(frames.last(), Some((FrameKind::Par, _))) {
                return Err(err(
                    lineno,
                    "'and' is only valid inside an open 'par' frame".to_string(),
                ));
            }
            d.items.push(SeqItem::FrameElse {
                label: clean_label_1line(rest.trim()),
            });
            continue;
        }
        if let Some(rest) = strip_keyword(line, "end") {
            if !rest.trim().is_empty() {
                return Err(err(lineno, format!("unexpected text after 'end': '{}'", rest.trim())));
            }
            if frames.pop().is_none() {
                return Err(err(
                    lineno,
                    "'end' without an open loop/opt/alt/par frame".to_string(),
                ));
            }
            d.items.push(SeqItem::FrameEnd);
            continue;
        }
        // Known sequence elements we don't support yet: explicit
        // error instead of a confusing message-parse failure.
        const UNSUPPORTED: &[&str] = &[
            "box", "rect", "critical", "option", "break", "create", "destroy", "title",
            "links", "link", "properties", "details",
        ];
        if let Some(kw) = UNSUPPORTED.iter().find(|k| strip_keyword(line, k).is_some()) {
            return Err(err(lineno, format!("sequence element '{}' is not supported yet", kw)));
        }
        parse_seq_message(&mut d, line, lineno, &mut depth)?;
    }
    if let Some(&(kind, opened)) = frames.last() {
        return Err(err(
            opened,
            format!("'{}' frame is never closed with 'end'", kind.keyword()),
        ));
    }
    Ok(d)
}

fn grow(depth: &mut Vec<usize>, p: usize) {
    if depth.len() <= p {
        depth.resize(p + 1, 0);
    }
}

/// `participant A` / `participant A as Pretty Label` (also `actor`).
/// Re-declaring an id updates its label and actor flag in place, so
/// declarations may follow implicit first mentions.
fn parse_participant(
    d: &mut SequenceDiagram,
    rest: &str,
    actor: bool,
    lineno: usize,
) -> Result<(), ParseError> {
    let mut cur = Cur::new(rest);
    let id = seq_ident(&mut cur, lineno)?;
    cur.skip_ws();
    let label = if cur.at_end() {
        None
    } else if let Some(after) = strip_keyword(cur.rest(), "as") {
        let l = clean_label_1line(after.trim());
        if l.is_empty() {
            return Err(err(lineno, "expected a label after 'as'".to_string()));
        }
        Some(l)
    } else {
        return Err(err(
            lineno,
            format!("expected 'as <label>' after the id, found: '{}'", cur.rest()),
        ));
    };
    let p = d.ensure_participant(&id);
    if let Some(l) = label {
        d.participants[p].label = l;
    }
    d.participants[p].actor = actor;
    Ok(())
}

/// A participant id: alphanumerics + `_` (`-` would collide with
/// the message operators).
fn seq_ident(cur: &mut Cur<'_>, lineno: usize) -> Result<String, ParseError> {
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
            format!("expected a participant id, found: '{}'", cur.rest()),
        ));
    }
    Ok(cur.s[start..cur.pos].to_string())
}

/// A whole-string participant reference (activate / deactivate /
/// note targets). Unknown ids are created implicitly.
fn seq_participant(
    d: &mut SequenceDiagram,
    rest: &str,
    lineno: usize,
) -> Result<usize, ParseError> {
    let mut cur = Cur::new(rest);
    let id = seq_ident(&mut cur, lineno)?;
    cur.skip_ws();
    if !cur.at_end() {
        return Err(err(
            lineno,
            format!("unexpected text after participant id: '{}'", cur.rest()),
        ));
    }
    Ok(d.ensure_participant(&id))
}

/// The tail of a note line (after the `note` keyword):
/// `over A[,B]: text` / `left of A: text` / `right of A: text`.
/// All keywords are case-insensitive, mermaid-style.
fn parse_seq_note(
    d: &mut SequenceDiagram,
    rest: &str,
    lineno: usize,
) -> Result<SeqItem, ParseError> {
    let Some(colon) = rest.find(':') else {
        return Err(err(lineno, "note needs ': text'".to_string()));
    };
    let (place, text) = (rest[..colon].trim(), clean_label_1line(rest[colon + 1..].trim()));
    if text.is_empty() {
        return Err(err(lineno, "note text is empty".to_string()));
    }
    let side = if let Some(ids) = strip_keyword(place, "over") {
        let mut it = ids.split(',').map(str::trim);
        let a = it
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| err(lineno, "'note over' needs a participant".to_string()))?;
        let b = it.next();
        if it.next().is_some() {
            return Err(err(
                lineno,
                "'note over' takes at most two participants".to_string(),
            ));
        }
        let ai = seq_participant(d, a, lineno)?;
        let bi = match b {
            Some(s) if !s.is_empty() => Some(seq_participant(d, s, lineno)?),
            Some(_) => {
                return Err(err(lineno, "empty participant after ',' in 'note over'".to_string()))
            }
            None => None,
        };
        // `Note over A,A` is a note over a single participant, not a
        // zero-width span between A and itself.
        let bi = bi.filter(|&second| second != ai);
        NoteSide::Over(ai, bi)
    } else {
        let (left, after) = if let Some(r) = strip_keyword(place, "left") {
            (true, r)
        } else if let Some(r) = strip_keyword(place, "right") {
            (false, r)
        } else {
            return Err(err(
                lineno,
                format!("expected 'over' / 'left of' / 'right of', found: '{}'", place),
            ));
        };
        let id = strip_keyword(after.trim(), "of")
            .ok_or_else(|| err(lineno, "expected 'of' after left/right".to_string()))?;
        let p = seq_participant(d, id.trim(), lineno)?;
        if left {
            NoteSide::LeftOf(p)
        } else {
            NoteSide::RightOf(p)
        }
    };
    Ok(SeqItem::Note { side, text })
}

/// Message arrow operators, longest first: (token, dashed, head).
/// `->` / `-->` are headless lines, mermaid-style.
const SEQ_OPS: &[(&str, bool, SeqHead)] = &[
    ("-->>", true, SeqHead::Filled),
    ("-->", true, SeqHead::None),
    ("--x", true, SeqHead::Cross),
    ("--)", true, SeqHead::Async),
    ("->>", false, SeqHead::Filled),
    ("->", false, SeqHead::None),
    ("-x", false, SeqHead::Cross),
    ("-)", false, SeqHead::Async),
];

/// `A->>+B: text` — id, operator, optional `+`/`-` activation
/// shorthand, target id, `: text`.
fn parse_seq_message(
    d: &mut SequenceDiagram,
    line: &str,
    lineno: usize,
    depth: &mut Vec<usize>,
) -> Result<(), ParseError> {
    let mut cur = Cur::new(line);
    let from_id = seq_ident(&mut cur, lineno)?;
    cur.skip_ws();
    let found = SEQ_OPS.iter().find(|(op, _, _)| cur.rest().starts_with(op));
    let Some(&(op, dashed, head)) = found else {
        return Err(err(
            lineno,
            format!(
                "unknown message operator near: '{}' (expected one of \
                 ->> -->> -> --> -x --x -) --) )",
                cur.rest()
            ),
        ));
    };
    cur.pos += op.len();
    cur.skip_ws();
    // `+` activates the target, `-` deactivates the SENDER (mermaid
    // shorthand: `A->>+B:` starts B's bar, `B-->>-A:` ends B's).
    let (activate, deactivate) = if cur.eat("+") {
        (true, false)
    } else if cur.eat("-") {
        (false, true)
    } else {
        (false, false)
    };
    cur.skip_ws();
    let to_id = seq_ident(&mut cur, lineno)?;
    cur.skip_ws();
    if !cur.eat(":") {
        return Err(err(
            lineno,
            format!("expected ': text' after the message target, found: '{}'", cur.rest()),
        ));
    }
    let text = clean_label_1line(cur.rest().trim());
    let from = d.ensure_participant(&from_id);
    let to = d.ensure_participant(&to_id);
    grow(depth, from.max(to));
    if activate {
        depth[to] += 1;
    }
    if deactivate {
        if depth[from] == 0 {
            return Err(err(
                lineno,
                format!("'-' deactivates the sender, but '{}' is not activated", from_id),
            ));
        }
        depth[from] -= 1;
    }
    d.items.push(SeqItem::Message {
        from,
        to,
        text,
        dashed,
        head,
        activate,
        deactivate,
    });
    Ok(())
}


// ---------------------------------------------------------------
// Pie chart (`pie`)
// ---------------------------------------------------------------

/// Parse a `pie` chart. `header_line` is the 1-indexed line of the
/// `pie` header, which itself may carry `showData` and/or
/// `title …`. The body is an optional standalone `title …` line and
/// `"Quoted Label" : value` data rows (non-negative numbers). A
/// duplicate label updates the existing slice in place — the LAST
/// value wins, matching `Graph::ensure_node`'s latest-wins rule.
fn parse_pie(source: &str, header_line: usize) -> Result<PieChart, ParseError> {
    let mut d = PieChart::default();

    // Header forms: `pie`, `pie showData`, `pie title T`,
    // `pie showData title T` (title consumes the rest of the line).
    let header = source.lines().nth(header_line - 1).unwrap_or("").trim();
    let mut rest = strip_keyword(header, "pie").unwrap_or("").trim();
    if let Some(r) = strip_keyword(rest, "showData") {
        d.show_data = true;
        rest = r.trim();
    }
    if let Some(r) = strip_keyword(rest, "title") {
        set_pie_title(&mut d, r, header_line)?;
    } else if !rest.is_empty() {
        return Err(err(
            header_line,
            format!("unexpected text after 'pie': '{}' (expected showData and/or title)", rest),
        ));
    }

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if let Some(r) = strip_keyword(line, "title") {
            set_pie_title(&mut d, r, lineno)?;
            continue;
        }
        let (label, value) = parse_pie_row(line, lineno)?;
        match d.slices.iter_mut().find(|s| s.label == label) {
            Some(s) => s.value = value, // duplicate label: last value wins
            None => d.slices.push(PieSlice { label, value }),
        }
    }
    Ok(d)
}

fn set_pie_title(d: &mut PieChart, text: &str, lineno: usize) -> Result<(), ParseError> {
    let t = text.trim();
    if t.is_empty() {
        return Err(err(lineno, "title needs text".to_string()));
    }
    d.title = Some(t.to_string());
    Ok(())
}

/// One data row: `"Quoted Label" : <non-negative number>`.
fn parse_pie_row(line: &str, lineno: usize) -> Result<(String, f64), ParseError> {
    let mut cur = Cur::new(line);
    if !cur.eat("\"") {
        return Err(err(
            lineno,
            format!("pie data row needs a quoted label, found: '{}'", line),
        ));
    }
    let label = cur
        .take_until("\"")
        .ok_or_else(|| err(lineno, "pie label quote is never closed".to_string()))?;
    cur.skip_ws();
    if !cur.eat(":") {
        return Err(err(
            lineno,
            format!("expected ':' after pie label \"{}\"", label),
        ));
    }
    let vs = cur.rest().trim();
    let value: f64 = vs
        .parse()
        .map_err(|_| err(lineno, format!("invalid pie value: '{}'", vs)))?;
    if !value.is_finite() || value < 0.0 {
        return Err(err(
            lineno,
            format!("pie value must be a non-negative number, got '{}'", vs),
        ));
    }
    Ok((label, value))
}

// ---------------------------------------------------------------
// Mindmap (`mindmap`)
// ---------------------------------------------------------------

/// Parse a mindmap: source INDENTATION builds a single-rooted tree.
/// Each line is one node; deeper indent = child of the nearest
/// shallower line. Node text may be wrapped to pick a shape
/// (`[..]` square, `(..)` rounded, `((..))` circle, `{{..}}` hexagon,
/// `))..((` bang, `)..(` cloud); an optional leading id is dropped.
/// `::icon(..)` and `:::class` decoration lines are ignored.
fn parse_mindmap(source: &str, header_line: usize) -> Result<Mindmap, ParseError> {
    let mut d = Mindmap::default();
    // Stack of open ancestors as (indent, node index), shallow -> deep.
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        // Decoration lines attach to the previous node — we don't draw
        // icons/classes, so skip them without creating a node.
        if trimmed.starts_with("::icon(") || trimmed.starts_with(":::") {
            continue;
        }

        let indent = mind_indent(raw);
        let (text, shape) = parse_mind_node(trimmed);
        if text.is_empty() {
            continue;
        }

        // Parent = nearest ancestor with strictly smaller indent.
        while matches!(stack.last(), Some(&(ind, _)) if ind >= indent) {
            stack.pop();
        }
        let mut parent = stack.last().map(|&(_, idx)| idx);
        // A single root is required; a stray shallow line after the
        // root is forgiven by hanging it off the root instead of
        // creating a second one.
        if parent.is_none() && !d.nodes.is_empty() {
            parent = Some(0);
        }

        let idx = d.nodes.len();
        let depth = parent.map_or(0, |p| d.nodes[p].depth + 1);
        if depth >= MAX_NEST_DEPTH {
            return Err(err(
                lineno,
                format!("mindmap nesting too deep (max {MAX_NEST_DEPTH} levels)"),
            ));
        }
        // A node's colored branch is its depth-1 ancestor; the root
        // has none. depth-1 nodes seed their own branch.
        let branch = match depth {
            0 => None,
            1 => Some(idx),
            _ => d.nodes[parent.unwrap()].branch,
        };
        d.nodes.push(MindNode {
            text,
            shape,
            parent,
            children: Vec::new(),
            depth,
            branch,
        });
        if let Some(p) = parent {
            d.nodes[p].children.push(idx);
        }
        stack.push((indent, idx));
    }

    if d.nodes.is_empty() {
        return Err(err(
            header_line,
            "mindmap has no nodes (expected an indented tree under the header)".to_string(),
        ));
    }
    Ok(d)
}

/// Indent width of a raw line: leading spaces count 1, tabs count 4.
fn mind_indent(raw: &str) -> usize {
    raw.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

/// Split a trimmed mindmap line into (display text, shape). A wrapper
/// picks the shape and its inner text becomes the label; any leading
/// id (adjacent to the wrapper) is discarded. `<br>` becomes a newline.
fn parse_mind_node(t: &str) -> (String, MindShape) {
    // Longest / most specific delimiters first. A wrapper only counts
    // when its opener is adjacent to the id (not preceded by a space),
    // so ordinary text ending in a parenthetical stays plain text.
    const WRAPS: &[(&str, &str, MindShape)] = &[
        ("((", "))", MindShape::Circle),
        ("))", "((", MindShape::Bang),
        ("{{", "}}", MindShape::Hexagon),
        ("[", "]", MindShape::Square),
        (")", "(", MindShape::Cloud),
        ("(", ")", MindShape::Rounded),
    ];
    for &(open, close, shape) in WRAPS {
        if let Some(oi) = t.find(open) {
            // The prefix before the opener is the node's optional id,
            // which cannot contain whitespace. If it does, this isn't an
            // `id + wrapper` — the whole line is plain text (e.g.
            // "call foo(x)" must stay literal, not become "x").
            let prefix_is_id = !t[..oi].contains(char::is_whitespace);
            let inner_start = oi + open.len();
            let has_close = t.ends_with(close) && t.len() >= inner_start + close.len();
            if prefix_is_id && has_close {
                let inner = &t[inner_start..t.len() - close.len()];
                return (mind_br(inner), shape);
            }
        }
    }
    (mind_br(t), MindShape::Rounded)
}

/// Normalise a mindmap label: `<br>` / `<br/>` / `<br />` -> newline,
/// then trim each line. Empty result is possible (caller skips it).
fn mind_br(s: &str) -> String {
    let mut out = s.to_string();
    for br in ["<br/>", "<br />", "<br>"] {
        out = out.replace(br, "\n");
    }
    out.split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------
// User journey (`journey`)
// ---------------------------------------------------------------

/// Parse a user-journey diagram: `title …`, `section …`, and task rows
/// `Name: score: Actor1, Actor2`. The score is clamped to 1..=5. Tasks
/// before the first `section` go into a leading unnamed section.
fn parse_journey(source: &str, header_line: usize) -> Result<Journey, ParseError> {
    let mut d = Journey::default();
    // Ensure there is always a current section to push tasks into.
    let ensure_section = |d: &mut Journey| {
        if d.sections.is_empty() {
            d.sections.push(JourneySection {
                name: String::new(),
                tasks: Vec::new(),
            });
        }
    };

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if let Some(rest) = strip_keyword(line, "title") {
            let t = rest.trim();
            if !t.is_empty() {
                d.title = Some(t.to_string());
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "section") {
            let name = rest.trim();
            if name.is_empty() {
                return Err(err(lineno, "section needs a name".to_string()));
            }
            d.sections.push(JourneySection {
                name: name.to_string(),
                tasks: Vec::new(),
            });
            continue;
        }
        // Task row: `Name: score[: Actor1, Actor2]`.
        let mut parts = line.splitn(3, ':');
        let name = parts.next().unwrap_or("").trim();
        let score_str = parts.next().map(str::trim);
        let actors_str = parts.next().map(str::trim).unwrap_or("");
        let Some(score_str) = score_str else {
            return Err(err(
                lineno,
                format!("journey task needs a score: '{}: <1-5>'", name),
            ));
        };
        if name.is_empty() {
            return Err(err(lineno, "journey task needs a name".to_string()));
        }
        let score: u8 = score_str
            .parse()
            .map_err(|_| err(lineno, format!("invalid journey score: '{}'", score_str)))?;
        let score = score.clamp(1, 5);
        // Resolve actors to stable indices (dedup, first-appearance order).
        let mut actor_ids = Vec::new();
        for a in actors_str.split(',') {
            let a = a.trim();
            if a.is_empty() {
                continue;
            }
            let id = d.actors.iter().position(|x| x == a).unwrap_or_else(|| {
                d.actors.push(a.to_string());
                d.actors.len() - 1
            });
            if !actor_ids.contains(&id) {
                actor_ids.push(id);
            }
        }
        ensure_section(&mut d);
        d.sections.last_mut().unwrap().tasks.push(JourneyTask {
            name: name.to_string(),
            score,
            actors: actor_ids,
        });
    }
    Ok(d)
}

// ---------------------------------------------------------------
// State diagram (`stateDiagram-v2` / `stateDiagram`)
// ---------------------------------------------------------------

/// Parse a state diagram straight onto a [`Graph`]: state = rounded
/// node, transition = labelled edge, composite `state X { ... }` =
/// nested subgraph, `[*]` = per-scope start/end pseudostate. The
/// whole flowchart layout/SVG/drag pipeline is reused unchanged.
fn parse_state(source: &str, header_line: usize) -> Result<Graph, ParseError> {
    let mut g = Graph::default();

    // Pre-scan composite ids so a transition may target a composite
    // declared LATER (`A --> Grp` before `state Grp {`) — the same
    // forward-reference courtesy the flowchart parser gives.
    let mut sub_ids: HashMap<String, usize> = HashMap::new();
    {
        let mut idx = 0usize;
        for raw in source.lines() {
            if let Some(rest) = strip_keyword(raw.trim(), "state") {
                if let Some(head) = rest.trim().strip_suffix('{') {
                    if let Ok((id, _, _)) = state_decl_head(head.trim(), 0) {
                        sub_ids.entry(id).or_insert(idx);
                        idx += 1;
                    }
                }
            }
        }
    }

    // Open composite blocks: (subgraph index, opening line).
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut note_block = false;
    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        let line = match find_unquoted(line, "%%") {
            Some(p) => line[..p].trim(),
            None => line,
        };
        if line.is_empty() {
            continue;
        }
        if note_block {
            if line.eq_ignore_ascii_case("end note") {
                note_block = false;
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "direction") {
            let d = parse_direction(rest.trim(), lineno)?;
            match stack.last() {
                Some(&(si, _)) => g.subgraphs[si].direction = Some(d),
                None => g.direction = d,
            }
            continue;
        }
        if line == "}" {
            if stack.pop().is_none() {
                return Err(err(lineno, "'}' without an open state block".to_string()));
            }
            continue;
        }
        // Notes are accepted and skipped (not rendered yet):
        // `note left of X : text` is one line; without a ':' it is a
        // block that runs until `end note`.
        if let Some(rest) = strip_keyword(line, "note") {
            if !rest.contains(':') {
                note_block = true;
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "state") {
            let rest = rest.trim();
            let (opens, head) = match rest.strip_suffix('{') {
                Some(h) => (true, h.trim()),
                None => (false, rest),
            };
            let (id, label, shape) = state_decl_head(head, lineno)?;
            if opens {
                // Composite: order of creation matches the pre-scan.
                let si = g.subgraphs.len();
                g.subgraphs.push(Subgraph {
                    id: id.clone(),
                    title: label.unwrap_or_else(|| id.clone()),
                    nodes: Vec::new(),
                    parent: stack.last().map(|&(s, _)| s),
                    direction: None,
                });
                stack.push((si, lineno));
            } else {
                state_node(&mut g, &stack, &id, label, Some(shape));
            }
            continue;
        }
        // Transition: `A --> B [: label]`, either end may be `[*]`
        // or a composite id.
        if let Some(apos) = line.find("-->") {
            let lhs = line[..apos].trim();
            let rest = line[apos + 3..].trim();
            let (rhs, label) = match rest.split_once(':') {
                Some((r, l)) => (r.trim(), Some(clean_label_1line(l.trim()))),
                None => (rest, None),
            };
            if lhs.is_empty() || rhs.is_empty() {
                return Err(err(lineno, "a transition needs both endpoints".to_string()));
            }
            let from = state_endpoint(&mut g, &stack, &sub_ids, lhs, false, lineno)?;
            let to = state_endpoint(&mut g, &stack, &sub_ids, rhs, true, lineno)?;
            match (from, to) {
                (End::Node(a), End::Node(b)) => g.add_edge(a, b, label, EdgeKind::Arrow),
                (from, to) => g.sub_edges.push(SubEdge {
                    from,
                    to,
                    label,
                    kind: EdgeKind::Arrow,
                }),
            }
            continue;
        }
        // `id : description` — description lines replace the default
        // label (the id) on first use and stack below it afterwards.
        if let Some((id_part, desc)) = line.split_once(':') {
            let (id_part, desc) = (id_part.trim(), desc.trim());
            if is_class_ident(id_part) && !desc.is_empty() {
                let idx = state_node(&mut g, &stack, id_part, None, None);
                let node = &mut g.nodes[idx];
                if node.label == node.id {
                    node.label = clean_label_1line(desc);
                } else {
                    node.label.push('\n');
                    node.label.push_str(&clean_label_1line(desc));
                }
                continue;
            }
        }
        // A bare id declares a state on its own line.
        if is_class_ident(line) {
            state_node(&mut g, &stack, line, None, None);
            continue;
        }
        return Err(err(lineno, format!("unrecognised state line: '{}'", line)));
    }
    if let Some(&(si, oln)) = stack.last() {
        return Err(err(
            oln,
            format!("state block '{}' is never closed with '}}'", g.subgraphs[si].id),
        ));
    }
    Ok(g)
}

/// Head of a `state` declaration (everything after the keyword,
/// before an optional `{`): `"desc" as id`, `id`, or
/// `id <<choice|fork|join>>`. Returns (id, label override, shape).
fn state_decl_head(
    head: &str,
    lineno: usize,
) -> Result<(String, Option<String>, Shape), ParseError> {
    let head = head.trim();
    if let Some(rest) = head.strip_prefix('"') {
        let close = rest
            .find('"')
            .ok_or_else(|| err(lineno, "state title quote is never closed".to_string()))?;
        let desc = clean_label_1line(&rest[..close]);
        let id = strip_keyword(rest[close + 1..].trim(), "as")
            .map(str::trim)
            .filter(|s| is_class_ident(s))
            .ok_or_else(|| err(lineno, "a titled state needs `as <id>`".to_string()))?;
        return Ok((id.to_string(), Some(desc), Shape::Rounded));
    }
    let (id, tail) = match head.split_once(char::is_whitespace) {
        Some((a, b)) => (a, b.trim()),
        None => (head, ""),
    };
    if !is_class_ident(id) {
        return Err(err(lineno, format!("invalid state id: '{}'", id)));
    }
    let (shape, blank) = match tail {
        "" => (Shape::Rounded, false),
        "<<choice>>" => (Shape::Diamond, true),
        "<<fork>>" | "<<join>>" => (Shape::ForkBar, true),
        other => {
            return Err(err(
                lineno,
                format!("unknown state stereotype: '{}'", other),
            ))
        }
    };
    // Pseudostates draw no text; blank out the default id label.
    Ok((id.to_string(), blank.then(String::new), shape))
}

/// Look up / create a state node; a NEW node created inside an open
/// composite block is claimed by that block.
fn state_node(
    g: &mut Graph,
    stack: &[(usize, usize)],
    id: &str,
    label: Option<String>,
    shape: Option<Shape>,
) -> usize {
    let is_new = g.node_index(id).is_none();
    let idx = g.ensure_node(
        id,
        label,
        shape.or(if is_new { Some(Shape::Rounded) } else { None }),
    );
    if is_new {
        if let Some(&(si, _)) = stack.last() {
            g.subgraphs[si].nodes.push(idx);
        }
    }
    idx
}

/// One transition endpoint: `[*]` (per-scope start/end pseudostate),
/// a composite id (the cluster box itself), or a plain state.
fn state_endpoint(
    g: &mut Graph,
    stack: &[(usize, usize)],
    sub_ids: &HashMap<String, usize>,
    token: &str,
    is_target: bool,
    lineno: usize,
) -> Result<End, ParseError> {
    if token == "[*]" {
        let key = stack
            .last()
            .map(|&(s, _)| s.to_string())
            .unwrap_or_else(|| "root".to_string());
        let (id, shape) = if is_target {
            (format!("__end_{key}"), Shape::StateEnd)
        } else {
            (format!("__start_{key}"), Shape::StateStart)
        };
        return Ok(End::Node(state_node(
            g,
            stack,
            &id,
            Some(String::new()),
            Some(shape),
        )));
    }
    if !is_class_ident(token) {
        return Err(err(lineno, format!("invalid state id: '{}'", token)));
    }
    // A composite id wins over a plain node of the same name,
    // mermaid-style: the transition attaches to the cluster box.
    if let Some(&si) = sub_ids.get(token) {
        return Ok(End::Sub(si));
    }
    Ok(End::Node(state_node(g, stack, token, None, None)))
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
        for src in ["gantt\ntitle x", "timeline\nx"] {
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

    fn state(src: &str) -> crate::model::Graph {
        match parse_document(src).unwrap() {
            Document::State(g) => g,
            other => panic!("expected a state diagram, got {:?}", other),
        }
    }

    #[test]
    fn state_pseudostates_are_scoped_and_shaped() {
        let g = state(
            "stateDiagram-v2\n[*] --> A\nA --> [*]\n\
             state B {\n[*] --> Inner\nInner --> [*]\n}",
        );
        // Root and composite each get their OWN start/end nodes.
        let shapes: Vec<Shape> = g.nodes.iter().map(|n| n.shape).collect();
        assert_eq!(shapes.iter().filter(|s| **s == Shape::StateStart).count(), 2);
        assert_eq!(shapes.iter().filter(|s| **s == Shape::StateEnd).count(), 2);
        // Composite membership: its [*]s and Inner belong to B.
        assert_eq!(g.subgraphs.len(), 1);
        assert_eq!(g.subgraphs[0].nodes.len(), 3, "start + Inner + end");
        // Pseudostates draw no text.
        assert!(g.nodes.iter().filter(|n| n.shape == Shape::StateStart).all(|n| n.label.is_empty()));
    }

    #[test]
    fn state_descriptions_replace_then_stack() {
        let g = state("stateDiagram-v2\nIdle : waiting\nIdle : for input\nIdle --> Done");
        let idle = &g.nodes[g.node_index("Idle").unwrap()];
        assert_eq!(idle.label, "waiting\nfor input");
        assert_eq!(idle.shape, Shape::Rounded);
    }

    #[test]
    fn state_stereotypes_and_titles() {
        let g = state(
            "stateDiagram-v2\nstate \"Long name\" as ln\nstate c <<choice>>\n\
             state f <<fork>>\nln --> c\nc --> f",
        );
        assert_eq!(g.nodes[g.node_index("ln").unwrap()].label, "Long name");
        let c = &g.nodes[g.node_index("c").unwrap()];
        assert_eq!((c.shape, c.label.as_str()), (Shape::Diamond, ""));
        assert_eq!(g.nodes[g.node_index("f").unwrap()].shape, Shape::ForkBar);
    }

    #[test]
    fn state_transition_to_composite_becomes_sub_edge() {
        let g = state(
            "stateDiagram-v2\nA --> Grp : go\nstate Grp {\n[*] --> X\n}\ndirection LR",
        );
        assert_eq!(g.sub_edges.len(), 1, "forward ref to the composite box");
        assert!(matches!(g.sub_edges[0].to, End::Sub(0)));
        assert_eq!(g.sub_edges[0].label.as_deref(), Some("go"));
        assert_eq!(g.direction, crate::model::Direction::LR);
    }

    #[test]
    fn state_notes_are_skipped_and_v1_header_works() {
        let g = state(
            "stateDiagram\nA --> B\nnote right of A : hi\n\
             note left of B\nmulti\nline\nend note\nB --> A",
        );
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn state_errors_carry_line_numbers() {
        let e = parse_document("stateDiagram-v2\nstate X {\nA --> B").unwrap_err();
        assert_eq!(e.line, 2, "unclosed block reports its opening line");
        let e = parse_document("stateDiagram-v2\n}").unwrap_err();
        assert_eq!(e.line, 2);
        let e = parse_document("stateDiagram-v2\nstate y <<weird>>").unwrap_err();
        assert!(e.message.contains("stereotype"), "{}", e.message);
        let e = parse_document("stateDiagram-v2\nstate \"titled\"").unwrap_err();
        assert!(e.message.contains("as <id>"), "{}", e.message);
    }

    #[test]
    fn utf8_bom_is_stripped_for_every_diagram_type() {
        // Bug hunt: a BOM from Windows editors survived trim() (it is
        // not Unicode whitespace), broke header detection for all
        // five types, and produced "expected a node id, found:
        // 'flowchart TD'" with the BOM invisible in a terminal.
        for src in [
            "flowchart TD\nA --> B",
            "erDiagram\nA ||--o{ B : has",
            "classDiagram\nAnimal <|-- Dog",
            "sequenceDiagram\nA->>B: hi",
            "pie\n\"a\" : 1",
            "stateDiagram-v2\n[*] --> A",
        ] {
            let bom = format!("\u{feff}{src}");
            assert!(parse_document(&bom).is_ok(), "BOM must not break: {src}");
        }
        // The flowchart-only entry point gets the same courtesy.
        assert!(parse("\u{feff}A --> B").is_ok());
    }

    #[test]
    fn deeply_nested_subgraphs_error_instead_of_overflowing_the_stack() {
        // Before the guard, ~18k nested `subgraph` blocks overflowed the
        // stack in the layout pass (`arrange()` recurses one frame per
        // level). The parser now rejects anything past MAX_NEST_DEPTH
        // with a clean, line-numbered error — no crash.
        let n = MAX_NEST_DEPTH + 5;
        let mut s = String::from("flowchart TD\n");
        for i in 0..n {
            s.push_str(&format!("subgraph s{i}\n"));
        }
        s.push_str("A --> B\n");
        for _ in 0..n {
            s.push_str("end\n");
        }
        let e = parse(&s).expect_err("over-deep nesting must be rejected");
        assert!(e.message.contains("too deep"), "got: {}", e.message);
    }

    #[test]
    fn deeply_nested_mindmap_errors_instead_of_overflowing() {
        // Same hazard on the mindmap side (`weigh`/`place` recursion).
        let n = MAX_NEST_DEPTH + 5;
        let mut s = String::from("mindmap\n");
        for d in 0..n {
            s.push_str(&"  ".repeat(d + 1));
            s.push_str(&format!("n{d}\n"));
        }
        let e = parse_document(&s).expect_err("over-deep mindmap must be rejected");
        assert!(e.message.contains("too deep"), "got: {}", e.message);
    }

    #[test]
    fn nesting_within_the_limit_still_parses() {
        // A generous-but-sane 100 levels — far under the cap — must work,
        // so the guard never gets in a real diagram's way.
        let n = 100;
        let mut s = String::from("flowchart TD\n");
        for i in 0..n {
            s.push_str(&format!("subgraph s{i}\n"));
        }
        s.push_str("A --> B\n");
        for _ in 0..n {
            s.push_str("end\n");
        }
        assert!(parse(&s).is_ok(), "100-deep nesting is legitimate");
    }
}
