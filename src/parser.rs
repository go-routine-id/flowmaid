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

use crate::markdown;
use crate::model::{
    ArchEdge, ArchGroup, ArchService, ArchSide, Architecture, Attr, Card, Class, ClassDiagram,
    ClassRel, CommitKind, Direction, Document, EdgeAnimation, EdgeCurve, EdgeEnds, EdgeKind,
    EdgeMeta, End, ErDiagram, FrameKind, GitBranch, GitCommit, GitGraph, GitOrientation, Graph,
    Journey, JourneySection, JourneyTask, Key, LinkStyle, LinkTarget, Marker, Member, MindNode,
    MindShape, Mindmap, NodeStyle, NoteSide, PieChart, PieSlice, RelKind, Relation,
    SankeyAlignment, SankeyConfig, SankeyDiagram, SankeyLabelStyle, SankeyLink, SankeyLinkColor,
    SeqHead, SeqItem, SequenceDiagram, Shape, SubEdge, Subgraph, Timeline, TimelineDirection,
    TimelinePeriod, TimelineSection, Visibility,
};
use crate::shapedata;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

fn parse_direction(s: &str, lineno: usize) -> Result<Direction, ParseError> {
    match s.trim().to_uppercase().as_str() {
        "TD" | "TB" => Ok(Direction::TD),
        "LR" => Ok(Direction::LR),
        "RL" => Ok(Direction::RL),
        "BT" => Ok(Direction::BT),
        other => Err(err(lineno, format!("unknown direction: '{}'", other))),
    }
}

/// The orientation a `gitGraph` header names. Mermaid writes it with a
/// trailing colon (`gitGraph TB:`, and a bare `gitGraph:`); the
/// colon-less form flowmaid always took stays accepted.
fn parse_git_orientation(line: &str, lineno: usize) -> Result<GitOrientation, ParseError> {
    const KW: &str = "gitGraph";
    let rest = match line.get(..KW.len()) {
        Some(head) if head.eq_ignore_ascii_case(KW) => &line[KW.len()..],
        _ => line,
    };
    let rest = rest.trim();
    let rest = rest.strip_suffix(':').unwrap_or(rest).trim();
    if rest.is_empty() {
        return Ok(GitOrientation::LR);
    }
    match rest.to_uppercase().as_str() {
        "LR" => Ok(GitOrientation::LR),
        "TB" => Ok(GitOrientation::TB),
        "BT" => Ok(GitOrientation::BT),
        other => Err(err(
            lineno,
            format!("unknown gitGraph orientation: '{}'", other),
        )),
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
    // Trim the edges again AFTER folding `<br/>` — folding can
    // expose whitespace that the pre-fold trim couldn't see
    // (`"x <br/>"` → "x", not "x "). str::trim subsumes the old
    // newline-only trim.
    decode_entities(&normalize_breaks(inner)).trim().to_string()
}

/// Decode mermaid's escape entities: `#quot;` → `"` and numeric
/// `#NN;` → the character with that decimal code point. This is how
/// mermaid smuggles characters its grammar reserves (`"`, `|`, `#`)
/// into labels — and how [`crate::emit::to_mermaid`] writes them
/// back out losslessly. Anything not matching the pattern passes
/// through unchanged (`#f00`, `C#`, a lone `#`).
pub(crate) fn decode_entities(s: &str) -> String {
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
/// contexts without multi-line layout (edge labels) whose boxes are
/// single-line.
fn clean_label_1line(s: &str) -> String {
    clean_label(s).replace('\n', " ")
}

/// Lower a markdown string body (see [`crate::markdown`]): node labels
/// (`node`) keep their lines and wrap; edge labels and subgraph titles
/// are one line.
fn md_label(raw: &str, lineno: usize, node: bool) -> Result<String, ParseError> {
    markdown::lower(raw, node, !node).map_err(|m| err(lineno, m))
}

/// An edge label or subgraph title: a whole-label markdown string is
/// lowered to one line, anything else cleaned like before.
fn label_1line(s: &str, lineno: usize) -> Result<String, ParseError> {
    match markdown::md_quoted(s) {
        Some(inner) => md_label(inner, lineno, false),
        None => Ok(clean_label_1line(s)),
    }
}

/// Replace `<br>` variants (case-insensitive, optional `/` and
/// spaces) with `\n`. Everything else passes through unchanged.
pub(crate) fn normalize_breaks(s: &str) -> String {
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
/// mermaid.js accepts a leading YAML frontmatter block fenced by
/// `---`, carrying `title:` / `config:` / theme keys. flowmaid
/// doesn't interpret those yet, but files exported from mermaid-live
/// or GitHub almost always carry the block — so we accept-and-drop
/// it (the same posture as `%%{init}` config comments): every line
/// of the block is blanked to empty, which every diagram parser
/// already skips, and blanking (rather than removing) keeps every
/// later line number exact so error messages still point true.
///
/// The block must be the very first content — only blank lines may
/// precede the opening `---`, matching mermaid. Returns the source
/// untouched when there's no frontmatter; errors on an unterminated
/// block. Call it AFTER the BOM strip.
fn strip_frontmatter(source: &str) -> Result<Cow<'_, str>, ParseError> {
    // Matches mermaid's fence regex
    // `/^([^\S\n\r]*)-{3}\s*[\n\r](.*?)[\n\r]\1-{3}\s*[\n\r]+/s`:
    // the OPEN `---` is the very first line (indented by spaces/tabs
    // at most — no leading blank lines), and the CLOSE must sit at the
    // SAME indent (the `\1` backreference). So a `---` that is
    // indented differently — e.g. inside a `title: |` block scalar —
    // is body, not a fence, and never closes the block early.
    let mut lines = source.lines().enumerate();
    let Some((_, first)) = lines.next() else {
        return Ok(Cow::Borrowed(source));
    };
    let Some(open_indent) = fence_indent(first) else {
        return Ok(Cow::Borrowed(source)); // no frontmatter
    };
    let Some(close) = lines.find_map(|(i, l)| (fence_indent(l) == Some(open_indent)).then_some(i))
    else {
        return Err(err(
            1,
            "unterminated frontmatter — the leading '---' block is never closed with '---'"
                .to_string(),
        ));
    };
    // Blank the fence + body lines; line count (and thus every later
    // line number) is preserved.
    let out = source
        .lines()
        .enumerate()
        .map(|(i, l)| if i <= close { "" } else { l })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Cow::Owned(out))
}

/// If `line` is a YAML frontmatter fence — leading spaces/tabs, then
/// exactly `---`, then only trailing whitespace — return its leading
/// indent (so open and close can be required to match); else `None`.
fn fence_indent(line: &str) -> Option<&str> {
    let body = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - body.len()];
    (body.trim_end() == "---").then_some(indent)
}

/// Split one physical line into logical statements on top-level `;`
/// separators — a `;` inside a quoted label (`"…"`), an edge-label
/// `|…|`, or shape brackets (`[]`/`()`/`{}`) is literal text, not a
/// separator. A line with no top-level `;` yields itself unchanged.
/// Fragments are returned untrimmed.
fn split_statements(line: &str) -> Vec<&str> {
    if !line.contains(';') {
        return vec![line];
    }
    let mut out = Vec::new();
    // A quote is the outermost context (it protects brackets/pipes/`;`);
    // an edge-label `|…|` and shape brackets `[]/()/{}` are their own
    // literal contexts. A `|` only OPENS an edge label at top level —
    // a `|` inside brackets (`A[read|write]`, `A[Yes|No]`) is label
    // text, never a delimiter.
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut in_pipe = false;
    let mut start = 0usize;
    for (i, c) in line.char_indices() {
        if in_quote {
            if c == '"' {
                in_quote = false;
            }
            continue;
        }
        if in_pipe {
            match c {
                '"' => in_quote = true,
                '|' => in_pipe = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_quote = true,
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' if depth > 0 => depth -= 1,
            '|' if depth == 0 => in_pipe = true,
            ';' if depth == 0 => {
                out.push(&line[start..i]);
                start = i + 1; // ';' is one ASCII byte
            }
            _ => {}
        }
    }
    out.push(&line[start..]);
    out
}

/// The logical statements of a flowchart source as `(1-based physical
/// line number, trimmed fragment)`: blank lines and full-line `%%`
/// comments are dropped, every other line is split on top-level `;`,
/// and empty fragments are skipped. The subgraph pre-scan and the main
/// pass BOTH iterate this, so they classify every statement
/// identically — the no-drift invariant that keeps sub-edges bound to
/// the right box depends on it (`;` must never desync the two passes).
///
/// A shape-data block (`A@{ … }`, `e1@{ … }`) or a markdown string
/// (`"`…`"`) may span lines, as in mermaid: a line that leaves one open
/// is joined with the following lines — newlines kept, since they select the YAML block form, and
/// full-line `%%` comments dropped — and numbered by its first line.
fn statements(source: &str) -> Result<Vec<(usize, Cow<'_, str>)>, ParseError> {
    let mut out = Vec::new();
    let mut lines = source.lines().enumerate();
    while let Some((i, raw)) = lines.next() {
        let phys = raw.trim();
        if phys.is_empty() || phys.starts_with("%%") {
            continue;
        }
        if !leaves_open(raw) {
            for frag in split_statements(raw) {
                let f = frag.trim();
                if !f.is_empty() {
                    out.push((i + 1, Cow::Borrowed(f)));
                }
            }
            continue;
        }
        let mut buf = raw.to_string();
        while leaves_open(&buf) {
            let Some((_, next)) = lines.next() else {
                let what = if markdown::leaves_open(&buf) {
                    "markdown string '\"`' is never closed with '`\"'"
                } else {
                    "'@{' metadata block is never closed with '}'"
                };
                return Err(err(i + 1, what.to_string()));
            };
            if !next.trim_start().starts_with("%%") {
                buf.push('\n');
                buf.push_str(next);
            }
        }
        for frag in split_statements(&buf) {
            let f = frag.trim();
            if !f.is_empty() {
                out.push((i + 1, Cow::Owned(f.to_string())));
            }
        }
    }
    Ok(out)
}

/// Mermaid's edge metadata for one user edge id while parsing, merged
/// key by key as `id@{ … }` statements arrive (last write wins), and
/// resolved onto the edge once the whole source is read.
#[derive(Default)]
struct RawEdgeMeta {
    /// `animate`, by JavaScript truthiness (`yes` animates, `0` not).
    animate: bool,
    /// A truthy `animation` value, stringified.
    animation: Option<String>,
    /// A truthy `curve` value, stringified.
    curve: Option<String>,
}

impl RawEdgeMeta {
    /// Copy the keys an edge takes (mermaid flowDb `addVertex` edge
    /// branch); every other key is ignored.
    fn merge(&mut self, entries: &[(String, shapedata::Yaml)]) {
        for (k, v) in entries {
            match k.as_str() {
                "animate" => self.animate = v.truthy(),
                "animation" => self.animation = v.truthy().then(|| v.text()).flatten(),
                "curve" => self.curve = v.truthy().then(|| v.text()).flatten(),
                _ => {}
            }
        }
    }

    /// Mermaid's rendering rule: a truthy `animation` picks the class
    /// (`fast` / `slow`; any other value names a class with no CSS, so
    /// nothing moves), else a truthy `animate` means fast.
    fn resolve(&self) -> (Option<EdgeAnimation>, Option<EdgeCurve>) {
        let animation = match self.animation.as_deref() {
            Some("fast") => Some(EdgeAnimation::Fast),
            Some("slow") => Some(EdgeAnimation::Slow),
            Some(_) => None,
            None => self.animate.then_some(EdgeAnimation::Fast),
        };
        (
            animation,
            self.curve.as_deref().map(EdgeCurve::from_mermaid),
        )
    }
}

/// Parse-time state for mermaid's `@` syntax in a flowchart.
#[derive(Default)]
struct FlowMeta {
    /// Every user edge id seen so far, with its metadata.
    edges: HashMap<String, RawEdgeMeta>,
    /// Nodes whose `@{ shape: … }` is a borderless mermaid shape (`text`,
    /// braces, `bolt`): drawn without fill and stroke unless styled.
    borderless: HashSet<usize>,
    /// `linkStyle` state, keyed by mermaid's link index.
    links: LinkStyles,
    /// `click` tooltips and callback names by id, last write wins.
    /// Mermaid keeps tooltips in a map with no node check (so they
    /// reach a node declared later) and flowmaid keeps callbacks the
    /// same way; both resolve onto existing nodes at the end.
    click_tips: HashMap<String, String>,
    click_callbacks: HashMap<String, String>,
}

/// Where a link lives: flowmaid keeps node→node links in
/// `Graph::edges` and links touching a subgraph in `Graph::sub_edges`,
/// while mermaid numbers them all together in source order.
#[derive(Clone, Copy)]
enum LinkRef {
    Edge(usize),
    Sub(usize),
}

/// Per-link `linkStyle` state in mermaid's numbering (flowDb `edges`):
/// every link in source order, fan-outs expanded, `~~~` and links to a
/// subgraph included.
#[derive(Default)]
struct LinkStyles {
    order: Vec<LinkRef>,
    /// The link's own `interpolate`; a new link starts with the
    /// default in force when it is created (flowDb `addSingleLink`).
    interp: Vec<Option<String>>,
    /// The link's own style; a later `linkStyle <n>` replaces it.
    style: Vec<Option<LinkStyle>>,
    default_style: Option<LinkStyle>,
    default_interp: Option<String>,
}

impl LinkStyles {
    fn push(&mut self, r: LinkRef) {
        self.order.push(r);
        self.interp.push(self.default_interp.clone());
        self.style.push(None);
    }

    /// The edge metadata of link `k`, resolved: the default style
    /// overlaid with the link's own, which (like mermaid's appended
    /// `fill:none`) clears a default fill it doesn't set itself; and
    /// a curve from `linkStyle … interpolate` unless `@{ curve }` set
    /// one.
    fn resolve(&self, k: usize, meta: &mut EdgeMeta) {
        let mut st = self.default_style.clone().unwrap_or_default();
        if let Some(own) = &self.style[k] {
            st.apply_over(own);
            if own.fill.is_none() {
                st.fill = None;
            }
        }
        meta.style = st;
        if meta.curve.is_none() {
            meta.curve = self.interp[k]
                .as_ref()
                .or(self.default_interp.as_ref())
                .map(|c| EdgeCurve::from_mermaid(c));
        }
    }
}

/// Mermaid's `click` keyword and its sub-keywords (case-sensitive).
const CLICK_KW: &str = "click";
const CLICK_HREF: &str = "href";
const CLICK_CALL: &str = "call";

/// One parsed `click` statement.
struct ClickStmt {
    /// The comma-separated id list, as written.
    ids: String,
    /// `href` form: the url (entity codes decoded) and its target.
    link: Option<(String, Option<LinkTarget>)>,
    /// Callback form: the function name (arguments dropped).
    callback: Option<String>,
    tooltip: Option<String>,
}

/// A `"…"` string at the cursor (no escapes; mermaid's entity codes
/// decoded).
fn click_string(cur: &mut Cur<'_>, lineno: usize) -> Result<String, ParseError> {
    cur.bump();
    cur.take_until("\"")
        .map(|s| decode_entities(&s))
        .ok_or_else(|| err(lineno, "click: unclosed '\"' string".to_string()))
}

/// A bare word at the cursor: up to whitespace, a quote or `(`.
fn click_word<'a>(cur: &mut Cur<'a>) -> &'a str {
    let rest = cur.rest();
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '"' || c == '(')
        .unwrap_or(rest.len());
    cur.pos += end;
    &rest[..end]
}

/// An optional `"tooltip"`, then the end of the statement.
fn click_tail(cur: &mut Cur<'_>, lineno: usize) -> Result<Option<String>, ParseError> {
    cur.skip_ws();
    let tip = if cur.rest().starts_with('"') {
        Some(click_string(cur, lineno)?)
    } else {
        None
    };
    cur.skip_ws();
    if !cur.at_end() {
        return Err(err(
            lineno,
            format!("click: unexpected '{}' at the end", cur.rest()),
        ));
    }
    Ok(tip)
}

/// Parse the text after `click` — mermaid's forms, in mermaid's order:
/// `ID[,ID…] [href] "url" ["tooltip"] [_self|_blank|_parent|_top]`,
/// `ID callback ["tooltip"]` or `ID call callback(args) ["tooltip"]`.
/// Runs of whitespace are accepted where mermaid wants exactly one.
fn parse_click(rest: &str, lineno: usize) -> Result<ClickStmt, ParseError> {
    let mut cur = Cur::new(rest.trim());
    let ids = {
        let r = cur.rest();
        let end = r.find(char::is_whitespace).unwrap_or(r.len());
        cur.pos += end;
        r[..end].to_string()
    };
    if ids.is_empty() {
        return Err(err(lineno, "click needs a node id".to_string()));
    }
    cur.skip_ws();
    if cur.at_end() {
        return Err(err(
            lineno,
            format!("click: expected a url, callback or 'call' after '{ids}'"),
        ));
    }
    let save = cur.pos;
    let word = click_word(&mut cur);
    // Like mermaid's `"href"[\s]`, where the line's own newline counts.
    let href = word == CLICK_HREF && cur.peek().map_or(true, char::is_whitespace);
    if !href {
        cur.pos = save;
    }
    cur.skip_ws();
    if href || cur.rest().starts_with('"') {
        if !cur.rest().starts_with('"') {
            return Err(err(
                lineno,
                "click: expected a quoted url after 'href'".to_string(),
            ));
        }
        let url = click_string(&mut cur, lineno)?;
        cur.skip_ws();
        let tooltip = if cur.rest().starts_with('"') {
            Some(click_string(&mut cur, lineno)?)
        } else {
            None
        };
        cur.skip_ws();
        let target = if cur.at_end() {
            None
        } else {
            let t = click_word(&mut cur);
            let target = LinkTarget::from_mermaid(t).ok_or_else(|| {
                err(
                    lineno,
                    format!(
                        "click: link target must be _self, _blank, _parent or _top, found '{}'",
                        if t.is_empty() { cur.rest() } else { t }
                    ),
                )
            })?;
            cur.skip_ws();
            if cur.rest().starts_with('"') {
                return Err(err(
                    lineno,
                    "click: tooltip must come before the link target".to_string(),
                ));
            }
            if !cur.at_end() {
                return Err(err(
                    lineno,
                    format!("click: unexpected '{}' at the end", cur.rest()),
                ));
            }
            Some(target)
        };
        return Ok(ClickStmt {
            ids,
            link: Some((url, target)),
            callback: None,
            tooltip,
        });
    }
    let word = click_word(&mut cur);
    if word.is_empty() {
        return Err(err(
            lineno,
            format!(
                "click: expected a url, callback or 'call', found '{}'",
                cur.rest()
            ),
        ));
    }
    if LinkTarget::from_mermaid(word).is_some() {
        return Err(err(
            lineno,
            format!("click: expected a url, callback or 'call' before link target '{word}'"),
        ));
    }
    let name = if word == CLICK_CALL && cur.peek().map_or(true, char::is_whitespace) {
        cur.skip_ws();
        let name = click_word(&mut cur);
        if name.is_empty() {
            return Err(err(
                lineno,
                "click: 'call' needs a function name".to_string(),
            ));
        }
        cur.skip_ws();
        // The arguments are dropped; mermaid's `[^)]*` ends them at
        // the first `)`, quotes or not.
        if cur.eat("(") && cur.take_until(")").is_none() {
            return Err(err(
                lineno,
                format!("click: call {name}( is never closed with ')'"),
            ));
        }
        name
    } else {
        word
    };
    let name = name.to_string();
    let tooltip = click_tail(&mut cur, lineno)?;
    Ok(ClickStmt {
        ids,
        link: None,
        callback: Some(name),
        tooltip,
    })
}

/// Apply a `click` statement like mermaid's flowDb: a link lands only
/// on a node that already exists (a later or unknown id, a subgraph or
/// an edge id is ignored, and no node is created); a tooltip and a
/// callback are kept by id and resolve at the end. The url is stored
/// only if [`crate::model::sanitize_href`] passes it; a whitespace-only
/// or refused url clears the node's link, as mermaid's `undefined` does.
fn apply_click(g: &mut Graph, c: ClickStmt, fm: &mut FlowMeta) {
    for id in c.ids.split(',').map(str::trim).filter(|i| !i.is_empty()) {
        if let (Some((url, target)), Some(n)) = (&c.link, g.node_index(id)) {
            let l = g.links.entry(n).or_default();
            l.href = crate::model::sanitize_href(url);
            l.target = *target;
        }
        if let Some(t) = c.tooltip.as_ref().filter(|t| !t.is_empty()) {
            fm.click_tips.insert(id.to_string(), t.clone());
        }
        if let Some(cb) = &c.callback {
            fm.click_callbacks.insert(id.to_string(), cb.clone());
        }
    }
}

/// Mermaid's `linkStyle` keyword (case-sensitive there, and here).
const LINK_STYLE_KW: &str = "linkStyle";
/// The `linkStyle` target naming every link.
const LINK_STYLE_DEFAULT: &str = "default";
/// The optional curve clause of a `linkStyle` statement.
const LINK_STYLE_INTERPOLATE: &str = "interpolate";

/// One `linkStyle` statement (`rest` follows the keyword):
/// `default` or `N[,N…]` (no spaces), an optional `interpolate
/// <curve>`, then optional styles — at least one of the two. Indices
/// are checked against the links defined so far, as in mermaid; a
/// later `linkStyle` for the same index replaces the earlier style.
fn parse_link_style(rest: &str, lineno: usize, ls: &mut LinkStyles) -> Result<(), ParseError> {
    let rest = rest.trim();
    let (targets, tail) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, ""), |(a, b)| (a, b.trim_start()));
    let positions: Option<Vec<usize>> = if targets == LINK_STYLE_DEFAULT {
        None
    } else {
        let mut v = Vec::new();
        for n in targets.split(',') {
            if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
                return Err(err(
                    lineno,
                    format!(
                        "linkStyle expects 'default' or a link index list like 0,2 \
                         (no spaces), found '{targets}'"
                    ),
                ));
            }
            // An index too large for usize is out of bounds anyway.
            let i = n.parse::<usize>().unwrap_or(usize::MAX);
            if i >= ls.order.len() {
                return Err(err(
                    lineno,
                    format!(
                        "The index {n} for linkStyle is out of bounds. Valid indices for \
                         linkStyle are between 0 and {}. (Help: Ensure that the index is \
                         within the range of existing edges.)",
                        ls.order.len() as i64 - 1
                    ),
                ));
            }
            v.push(i);
        }
        Some(v)
    };
    let (interp, props) = match strip_keyword(tail, LINK_STYLE_INTERPOLATE)
        .filter(|_| tail.starts_with(LINK_STYLE_INTERPOLATE))
    {
        Some(r) => {
            let r = r.trim_start();
            let (name, props) = r
                .split_once(char::is_whitespace)
                .map_or((r, ""), |(a, b)| (a, b.trim_start()));
            if name.is_empty() {
                return Err(err(
                    lineno,
                    "linkStyle 'interpolate' needs a curve name".to_string(),
                ));
            }
            (Some(name.to_string()), props)
        }
        None => (None, tail),
    };
    let style = if props.is_empty() {
        None
    } else {
        Some(parse_link_props(props, lineno)?)
    };
    if interp.is_none() && style.is_none() {
        return Err(err(
            lineno,
            "linkStyle needs styles or 'interpolate <curve>'".to_string(),
        ));
    }
    match positions {
        None => {
            if interp.is_some() {
                ls.default_interp = interp;
            }
            if style.is_some() {
                ls.default_style = style;
            }
        }
        Some(v) => {
            for i in v {
                if interp.is_some() {
                    ls.interp[i].clone_from(&interp);
                }
                if style.is_some() {
                    ls.style[i].clone_from(&style);
                }
            }
        }
    }
    Ok(())
}

/// Split style properties on commas at paren depth 0 only, so CSS
/// function values (`rgb(255,0,0)`) survive as one property. An
/// unbalanced '(' would otherwise swallow every later property
/// silently — it is an error instead.
fn split_props(s: &str, lineno: usize) -> Result<Vec<&str>, ParseError> {
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
        return Err(err(
            lineno,
            format!("unbalanced '(' in style properties: '{s}'"),
        ));
    }
    items.push(&s[start..]);
    Ok(items)
}

/// Widest `stroke-width` (px) accepted: far past any drawable line,
/// and small enough that the SVG never carries a 300-digit number.
const MAX_STROKE_WIDTH: f64 = 1000.0;

/// Whether `w` is a `stroke-width` the parser accepts (and so the only
/// kind `to_mermaid` may write).
pub(crate) fn stroke_width_ok(w: f64) -> bool {
    w.is_finite() && !w.is_sign_negative() && w <= MAX_STROKE_WIDTH
}

/// A `stroke-width` value: a number with an optional `px`, from 0 to
/// [`MAX_STROKE_WIDTH`]. Negative (`-0` included), non-finite and huge
/// widths are errors — they would write invalid or absurd SVG. Shared
/// by node `style`/`classDef` and `linkStyle`.
fn parse_stroke_width(v: &str, lineno: usize) -> Result<f64, ParseError> {
    v.trim_end_matches("px")
        .trim()
        .parse()
        .ok()
        .filter(|&n| stroke_width_ok(n))
        .ok_or_else(|| {
            err(
                lineno,
                format!("invalid stroke-width: '{v}' (expected 0 to {MAX_STROKE_WIDTH}px)"),
            )
        })
}

/// `linkStyle` properties: `stroke`, `stroke-width`,
/// `stroke-dasharray`, `fill` and `color`. Unknown keys and empty
/// values are ignored, and so is an item without `:` — mermaid splits
/// `stroke-dasharray:5,5` at the comma and drops the bare `5`, so a
/// common real-world spelling keeps rendering (as dash `5`).
fn parse_link_props(s: &str, lineno: usize) -> Result<LinkStyle, ParseError> {
    let mut st = LinkStyle::default();
    for item in split_props(s, lineno)? {
        let Some((k, v)) = item.trim().split_once(':') else {
            continue;
        };
        let v = v.trim();
        if v.is_empty() {
            continue;
        }
        let slot = match k.trim() {
            "stroke" => &mut st.stroke,
            "stroke-dasharray" => &mut st.stroke_dasharray,
            "fill" => &mut st.fill,
            "color" => &mut st.color,
            "stroke-width" => {
                st.stroke_width = Some(parse_stroke_width(v, lineno)?);
                continue;
            }
            _ => continue,
        };
        *slot = Some(v.to_string());
    }
    Ok(st)
}

/// Mermaid's shape names (`A@{ shape: … }`, shapes.ts registry: short
/// names, aliases, and the internal aliases that pass its lowercase
/// gate) and the flowmaid [`Shape`] each draws as. A name mermaid
/// accepts but flowmaid has no geometry for falls back to the nearest
/// look-alike, so every valid mermaid diagram still renders.
const MERMAID_SHAPES: &[(&[&str], Shape)] = &[
    // Exact equivalents.
    (&["rect", "proc", "process", "rectangle"], Shape::Rect),
    (&["rounded", "event"], Shape::Rounded),
    (&["stadium", "terminal", "pill"], Shape::Stadium),
    (
        &[
            "fr-rect",
            "subprocess",
            "subproc",
            "framed-rectangle",
            "subroutine",
        ],
        Shape::Subroutine,
    ),
    (&["cyl", "db", "database", "cylinder"], Shape::Cylinder),
    (&["circle", "circ"], Shape::Circle),
    (
        &["dbl-circ", "double-circle", "doublecircle"],
        Shape::DoubleCircle,
    ),
    (&["diam", "decision", "diamond", "question"], Shape::Diamond),
    (&["hex", "hexagon", "prepare"], Shape::Hexagon),
    (&["lean-r", "lean-right", "in-out"], Shape::Parallelogram),
    (&["lean-l", "lean-left", "out-in"], Shape::ParallelogramAlt),
    (&["sm-circ", "start", "small-circle"], Shape::StateStart),
    (&["fr-circ", "stop", "framed-circle"], Shape::StateEnd),
    (&["fork", "join"], Shape::ForkBar),
    (&["f-circ", "junction", "filled-circle"], Shape::StateStart),
    // Fallbacks: no flowmaid geometry yet.
    (
        &["trap-b", "priority", "trapezoid-bottom", "trapezoid"],
        Shape::Parallelogram,
    ),
    (
        &["trap-t", "manual", "trapezoid-top", "inv-trapezoid"],
        Shape::ParallelogramAlt,
    ),
    (
        &[
            "odd",
            "doc",
            "document",
            "docs",
            "documents",
            "st-doc",
            "stacked-document",
            "lin-doc",
            "lined-document",
            "tag-doc",
            "tagged-document",
            "notch-rect",
            "card",
            "notched-rectangle",
            "div-rect",
            "div-proc",
            "divided-rectangle",
            "divided-process",
            "st-rect",
            "procs",
            "processes",
            "stacked-rectangle",
            "tag-rect",
            "tagged-rectangle",
            "tag-proc",
            "tagged-process",
            "win-pane",
            "internal-storage",
            "window-pane",
            "flag",
            "paper-tape",
            "sl-rect",
            "manual-input",
            "sloped-rectangle",
            "datastore",
            "data-store",
            "folder",
            "directory",
            "console",
            "browser",
        ],
        Shape::Rect,
    ),
    (
        &[
            "lin-rect",
            "lined-rectangle",
            "lined-process",
            "lin-proc",
            "shaded-process",
        ],
        Shape::Subroutine,
    ),
    (
        &[
            "delay",
            "half-rounded-rectangle",
            "bow-rect",
            "stored-data",
            "bow-tie-rectangle",
        ],
        Shape::Stadium,
    ),
    (
        &[
            "curv-trap",
            "curved-trapezoid",
            "display",
            "cloud",
            "person",
        ],
        Shape::Rounded,
    ),
    (
        &[
            "h-cyl",
            "das",
            "horizontal-cylinder",
            "lin-cyl",
            "disk",
            "lined-cylinder",
            "bucket",
        ],
        Shape::Cylinder,
    ),
    (
        &[
            "notch-pent",
            "loop-limit",
            "notched-pentagon",
            "hourglass",
            "collate",
            "bang",
        ],
        Shape::Hexagon,
    ),
    (
        &[
            "tri",
            "extract",
            "triangle",
            "flip-tri",
            "manual-file",
            "flipped-triangle",
        ],
        Shape::Diamond,
    ),
    (&["cross-circ", "summary", "crossed-circle"], Shape::Circle),
    (BORDERLESS_SHAPES, Shape::Rect),
    // Undocumented names mermaid's registry also accepts.
    (&["state", "composite", "bpmn-activity"], Shape::Rounded),
    (&["choice", "bpmn-gateway"], Shape::Diamond),
    (
        &["note", "icon", "bpmn-data", "bpmn-annotation"],
        Shape::Rect,
    ),
    (&["anchor"], Shape::StateStart),
    (
        &["bpmn-start", "bpmn-intermediate", "bpmn-boundary"],
        Shape::Circle,
    ),
    (&["bpmn-end"], Shape::DoubleCircle),
    (&["bpmn-data-store"], Shape::Cylinder),
];

/// Fill / stroke paint of a borderless mermaid shape.
const BORDERLESS_PAINT: &str = "none";

/// Mermaid shapes drawn as bare text (no box): a borderless rect here.
const BORDERLESS_SHAPES: &[&str] = &[
    "text",
    "brace",
    "comment",
    "brace-l",
    "brace-r",
    "braces",
    "bolt",
    "com-link",
    "lightning-bolt",
];

/// Resolve a mermaid `shape:` name with mermaid's own errors: a name
/// with uppercase or `_` fails its lowercase gate, an unknown one is
/// "No such shape". Returns the shape and whether it draws borderless.
fn mermaid_shape(name: &str, lineno: usize) -> Result<(Shape, bool), ParseError> {
    if name != name.to_lowercase() || name.contains('_') {
        return Err(err(
            lineno,
            format!("No such shape: {name}. Shape names should be lowercase."),
        ));
    }
    MERMAID_SHAPES
        .iter()
        .find(|(names, _)| names.contains(&name))
        .map(|&(_, shape)| (shape, BORDERLESS_SHAPES.contains(&name)))
        .ok_or_else(|| err(lineno, format!("No such shape: {name}.")))
}

/// Read a node's shape-data body (the cursor is just past `@{`) up to
/// and including its closing `}`.
fn node_meta_body(
    cur: &mut Cur<'_>,
    lineno: usize,
) -> Result<Vec<(String, shapedata::Yaml)>, ParseError> {
    let body = cur.rest();
    let end = shapedata::body_end(body).ok_or_else(|| {
        err(
            lineno,
            "'@{' metadata block is never closed with '}'".to_string(),
        )
    })?;
    cur.pos += end + 1;
    shape_data(&body[..end], lineno)
}

/// `labelType` values that keep a metadata label plain text; any other
/// value, or none, makes it markdown (mermaid 12's flowDb
/// `sanitizeNodeLabelType`).
const PLAIN_LABEL_TYPES: &[&str] = &["string", "text"];

/// Apply mermaid node metadata (flowDb `addVertex`): a truthy `shape`
/// must name a registry shape, a truthy `label` replaces the label.
/// As in mermaid 12 the label is markdown (lowered and wrapped like a
/// ``"`…`"`` node label) unless `labelType` is `string` or `text`,
/// which keep it plain (cleaned like a bracket label). `icon`, `img`,
/// `form`, `pos`, `w`, `h` and `constraint` are accepted but not drawn — and,
/// unlike mermaid, an icon/image node keeps its id as the label, since
/// no icon replaces it. Unknown keys are ignored.
fn apply_node_meta(
    g: &mut Graph,
    n: usize,
    entries: &[(String, shapedata::Yaml)],
    fm: &mut FlowMeta,
    lineno: usize,
) -> Result<(), ParseError> {
    use shapedata::Yaml;
    let plain = entries
        .iter()
        .find(|(k, _)| k == "labelType")
        .is_some_and(|(_, v)| matches!(v, Yaml::Str(t) if PLAIN_LABEL_TYPES.contains(&t.as_str())));
    for (k, v) in entries {
        match (k.as_str(), v) {
            ("shape", Yaml::Str(name)) if !name.is_empty() => {
                let (shape, borderless) = mermaid_shape(name, lineno)?;
                g.nodes[n].shape = shape;
                if borderless {
                    fm.borderless.insert(n);
                } else {
                    fm.borderless.remove(&n);
                }
            }
            ("shape", v) if v.truthy() => {
                return Err(err(lineno, "'shape' must be a shape name".to_string()));
            }
            ("label", v) if v.truthy() => {
                if let Some(text) = v.text() {
                    g.nodes[n].label = if plain {
                        clean_label(&text)
                    } else {
                        markdown::lower_text(&text, true, false)
                    };
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// A whole statement of the form `id@{ … }`: the id (mermaid's LINK_ID
/// charset — anything but whitespace, quotes, brackets and pipes), the
/// raw body, and whatever follows the closing `}`.
fn meta_statement(line: &str) -> Option<(&str, &str, &str)> {
    let o = shapedata::find_open(line)?;
    let id = &line[..o];
    if id.is_empty()
        || id
            .chars()
            .any(|c| c.is_whitespace() || LINK_ID_STOP.contains(c))
    {
        return None;
    }
    let body = &line[o + 2..];
    let e = shapedata::body_end(body)?;
    Some((id, &body[..e], &body[e + 1..]))
}

/// Characters that end mermaid's LINK_ID token (besides whitespace).
const LINK_ID_STOP: &str = "\"[](){}|";

/// Read a shape-data body into entries, with mermaid's lexer rewrite
/// of quoted newlines first; errors carry the statement's line.
fn shape_data(body: &str, lineno: usize) -> Result<Vec<(String, shapedata::Yaml)>, ParseError> {
    shapedata::parse(&shapedata::fold_quoted_newlines(body)).map_err(|m| err(lineno, m))
}

/// Mermaid's edge id before an operator (`A e1@--> B`, `A e1@ --> B`,
/// `A e1@-- text --> B`): consumes it and returns the id — the token
/// minus its first `@`, exactly as mermaid's `replace('@', '')` does —
/// only when an edge operator follows. The token is the longest run of
/// LINK_ID characters ending in an `@` that is not followed by `{` or
/// `"` (those start shape data / a string).
fn parse_link_id(cur: &mut Cur<'_>) -> Option<String> {
    let rest = cur.rest();
    let run = rest
        .find(|c: char| c.is_whitespace() || LINK_ID_STOP.contains(c))
        .unwrap_or(rest.len());
    let at = rest[..run]
        .char_indices()
        .rev()
        .filter(|&(i, c)| c == '@' && i > 0)
        .map(|(i, _)| i)
        .find(|&i| {
            rest[i + 1..]
                .chars()
                .next()
                .is_some_and(|c| c != '{' && c != '"')
        })?;
    let after = &rest[at + 1..];
    let op = after.trim_start();
    let body = op.strip_prefix(['<', 'x', 'o']).unwrap_or(op);
    let is_op = parse_edge_op(&mut Cur::new(op)).is_some()
        || ["--", "-.", "=="].iter().any(|o| body.starts_with(o));
    if !is_op {
        return None;
    }
    cur.pos += at + 1 + (after.len() - op.len());
    Some(rest[..=at].replacen('@', "", 1))
}

/// Header keyword of mermaid's ELK flowchart detector. Mermaid parses
/// it with the very same flowchart grammar and only switches the
/// layout engine; flowmaid has one layout, so it is an alias of
/// `flowchart` and the hint is dropped.
const FLOWCHART_ELK_HEADER: &str = "flowchart-elk";

/// Mermaid's accessibility statement keywords (case-sensitive there,
/// and here).
const ACC_TITLE_KW: &str = "accTitle";
const ACC_DESCR_KW: &str = "accDescr";

/// The two grammars mermaid uses for `accTitle` / `accDescr`.
#[derive(Clone, Copy, PartialEq)]
enum AccFlavor {
    /// jison diagrams (flowchart, state, class, sequence, ER, journey,
    /// timeline): a single-line value runs to the end of the line,
    /// `%%` included; a description has `\n\s+` folded to `\n`.
    Jison,
    /// langium diagrams (pie, gitGraph, architecture): a single-line
    /// value stops before `%%`, runs of spaces/tabs collapse, and a
    /// block description drops blank lines and per-line indentation.
    Langium,
}

/// Accessibility statements pulled out of a diagram body. The last
/// statement of each kind wins, as in mermaid; an empty value clears.
#[derive(Default)]
struct Acc {
    title: Option<String>,
    descr: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum AccHead {
    Title,
    Descr,
    DescrBlock,
}

/// Recognise an accessibility statement at the start of `s` (already
/// left-trimmed): `accTitle\s*:`, `accDescr\s*:` or `accDescr\s*{`.
/// Returns the kind and the byte offset just past the `:` / `{`. A
/// node id like `accTitle --> B` is not one.
fn acc_head(s: &str) -> Option<(AccHead, usize)> {
    let (kw, title) = if s.starts_with(ACC_TITLE_KW) {
        (ACC_TITLE_KW, true)
    } else if s.starts_with(ACC_DESCR_KW) {
        (ACC_DESCR_KW, false)
    } else {
        return None;
    };
    let after = &s[kw.len()..];
    let body = after.trim_start();
    let at = kw.len() + (after.len() - body.len()) + 1;
    match (body.chars().next()?, title) {
        (':', true) => Some((AccHead::Title, at)),
        (':', false) => Some((AccHead::Descr, at)),
        ('{', false) => Some((AccHead::DescrBlock, at)),
        _ => None,
    }
}

/// Where the first accessibility statement on `text` starts: its byte
/// offset, kind and value offset. With `split_semis` (flowchart) a
/// statement may also follow a top-level `;` (`A --> B; accTitle: T`).
fn find_acc(text: &str, split_semis: bool) -> Option<(usize, AccHead, usize)> {
    let at_start = |frag: &str, base: usize| {
        let t = frag.trim_start();
        let o = base + (frag.len() - t.len());
        acc_head(t).map(|(h, v)| (o, h, o + v))
    };
    if text.trim_start().starts_with("%%") {
        return None;
    }
    if !split_semis {
        return at_start(text, 0);
    }
    split_statements(text).into_iter().find_map(|frag| {
        // Fragments are sub-slices of `text`.
        let base = frag.as_ptr() as usize - text.as_ptr() as usize;
        at_start(frag, base)
    })
}

/// Normalise a single-line `accTitle` / `accDescr` value.
fn acc_line_value(raw: &str, flavor: AccFlavor) -> String {
    match flavor {
        AccFlavor::Jison => raw.trim().to_string(),
        AccFlavor::Langium => {
            let v = raw.find("%%").map_or(raw, |p| &raw[..p]);
            collapse_blanks(v.trim())
        }
    }
}

/// Normalise a multi-line `accDescr { … }` body.
fn acc_block_value(raw: &str, flavor: AccFlavor) -> String {
    match flavor {
        // commonDb `setAccDescription`: trim, then `\n\s+` → `\n`
        // (continuation indent goes, blank lines collapse).
        AccFlavor::Jison => {
            let mut out = String::new();
            let mut chars = raw.trim().chars().peekable();
            while let Some(c) = chars.next() {
                out.push(c);
                if c == '\n' {
                    while chars.peek().is_some_and(|c| c.is_whitespace()) {
                        chars.next();
                    }
                }
            }
            out
        }
        // langium value converter: per-line trim, blank lines
        // dropped, runs of spaces/tabs collapsed.
        AccFlavor::Langium => raw
            .lines()
            .map(|l| collapse_blanks(l.trim()))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Collapse every run of two or more spaces/tabs into one space.
fn collapse_blanks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_blank = false;
    for c in s.chars() {
        let blank = c == ' ' || c == '\t';
        if blank && prev_blank {
            continue;
        }
        out.push(if blank { ' ' } else { c });
        prev_blank = blank;
    }
    out
}

/// Pull mermaid's `accTitle:` / `accDescr:` / `accDescr { … }`
/// statements out of the lines AFTER `header_line` (1-based), so the
/// diagram parser never sees them. Every consumed line is blanked
/// rather than removed, keeping later line numbers exact; text after a
/// block's closing `}` stays on its line as ordinary statements.
/// Full-line `%%` comments inside a block are dropped, as mermaid's
/// comment pre-pass does. Returns the source untouched when there is
/// nothing to pull.
fn extract_acc(
    source: &str,
    header_line: usize,
    split_semis: bool,
    flavor: AccFlavor,
) -> Result<(Cow<'_, str>, Acc), ParseError> {
    let mut acc = Acc::default();
    let lines: Vec<&str> = source.lines().collect();
    if !lines
        .iter()
        .skip(header_line)
        .any(|l| l.contains(ACC_TITLE_KW) || l.contains(ACC_DESCR_KW))
    {
        return Ok((Cow::Borrowed(source), acc));
    }
    let set = |slot: &mut Option<String>, v: String| {
        *slot = (!v.is_empty()).then_some(v);
    };
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    // Flowchart only: the text of a statement left open by a multi-line
    // markdown string or `@{` block, joined as [`statements`] joins it.
    // Mermaid's acc rules fire only in its INITIAL lexer state, so a
    // line inside one is label / YAML text, and on the line that closes
    // it a statement may only follow a top-level `;` after the close.
    let mut carry: Option<String> = None;
    let mut i = header_line;
    while i < lines.len() {
        let mut text: &str = lines[i];
        let mut kept = String::new();
        if let Some(buf) = carry.as_mut() {
            if text.trim_start().starts_with("%%") {
                i += 1;
                continue;
            }
            let Some(k) = close_point(buf, text) else {
                buf.push('\n');
                buf.push_str(text);
                i += 1;
                continue;
            };
            let frags = split_statements(&text[k..]);
            let cut = if frags.len() > 1 {
                k + frags[0].len() + 1 // past the ';'
            } else {
                text.len()
            };
            kept.push_str(&text[..cut]);
            text = &text[cut..];
        }
        loop {
            let Some((o, head, v)) = find_acc(text, split_semis) else {
                kept.push_str(text);
                break;
            };
            kept.push_str(&text[..o]);
            match head {
                AccHead::Title => {
                    set(&mut acc.title, acc_line_value(&text[v..], flavor));
                    break;
                }
                AccHead::Descr => {
                    set(&mut acc.descr, acc_line_value(&text[v..], flavor));
                    break;
                }
                AccHead::DescrBlock => {
                    let body = &text[v..];
                    if let Some(p) = body.find('}') {
                        set(&mut acc.descr, acc_block_value(&body[..p], flavor));
                        text = &body[p + 1..];
                        continue;
                    }
                    let open = i;
                    let mut buf = body.to_string();
                    out[i] = std::mem::take(&mut kept);
                    loop {
                        i += 1;
                        let Some(&l) = lines.get(i) else {
                            return Err(err(
                                open + 1,
                                "'accDescr {' is never closed with '}'".to_string(),
                            ));
                        };
                        if let Some(p) = l.find('}') {
                            buf.push('\n');
                            buf.push_str(&l[..p]);
                            text = &l[p + 1..];
                            break;
                        }
                        if !l.trim_start().starts_with("%%") {
                            buf.push('\n');
                            buf.push_str(l);
                        }
                        out[i].clear();
                    }
                    set(&mut acc.descr, acc_block_value(&buf, flavor));
                }
            }
        }
        if split_semis {
            // A full-line `%%` comment never opens one (statements()
            // drops it before looking).
            let joined = match carry.take() {
                Some(buf) => Some(buf + "\n" + &kept),
                None if kept.trim_start().starts_with("%%") => None,
                None => Some(kept.clone()),
            };
            carry = joined.filter(|j| leaves_open(j));
        }
        out[i] = kept;
        i += 1;
    }
    Ok((Cow::Owned(out.join("\n")), acc))
}

/// Whether `s` leaves a shape-data block or a markdown string open, so
/// the statement continues on the next physical line.
fn leaves_open(s: &str) -> bool {
    shapedata::leaves_open(s) || markdown::leaves_open(s)
}

/// Where on `line` the multi-line construct left open by `buf` (the
/// statement's earlier lines) closes: the byte offset just past its
/// closing `}` or `` `" ``, or `None` when the whole line is inside it.
fn close_point(buf: &str, line: &str) -> Option<usize> {
    let joined = format!("{buf}\n{line}");
    let base = buf.len() + 1;
    line.char_indices()
        .filter(|&(_, c)| c == '}' || c == '"')
        .map(|(p, c)| p + c.len_utf8())
        .find(|&k| !leaves_open(&joined[..base + k]))
}

pub fn parse_document(source: &str) -> Result<Document, ParseError> {
    // Editor Windows (Notepad, PowerShell `>`) menyisipkan BOM UTF-8.
    // U+FEFF bukan whitespace Unicode, jadi lolos trim() dan membuat
    // header tak terdeteksi dengan error yang menyesatkan.
    let raw_source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let stripped = strip_frontmatter(raw_source)?;
    let source: &str = &stripped;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        return match diagram_type(line) {
            Some("erDiagram") => parse_er(source, i + 1).map(Document::Er),
            Some("classDiagram") => parse_class(source, i + 1).map(Document::Class),
            // Config is read from the ORIGINAL source (see sankey below).
            Some("sequenceDiagram") => parse_sequence(source, i + 1).map(|mut d| {
                apply_sequence_config(&mut d, raw_source);
                Document::Sequence(d)
            }),
            Some("pie") => parse_pie(source, i + 1).map(Document::Pie),
            Some("stateDiagram-v2") | Some("stateDiagram") => {
                parse_state(source, i + 1).map(Document::State)
            }
            Some("mindmap") => parse_mindmap(source, i + 1).map(Document::Mindmap),
            Some("journey") => parse_journey(source, i + 1).map(Document::Journey),
            Some("gitGraph") => parse_gitgraph(source, i + 1).map(Document::GitGraph),
            Some("architecture-beta") | Some("architecture") => {
                parse_architecture(source, i + 1).map(Document::Architecture)
            }
            Some("timeline") => parse_timeline(source, i + 1).map(Document::Timeline),
            // Config is read from the ORIGINAL source: the frontmatter
            // has already been blanked out of `source` by this point.
            Some("sankey-beta") | Some("sankey") => {
                parse_sankey(source, i + 1, sankey_config(raw_source)).map(Document::Sankey)
            }
            Some(t) => Err(err(
                i + 1,
                format!(
                    "diagram type '{}' is not supported yet (supported: {})",
                    t,
                    supported_types()
                ),
            )),
            // `source` here is already BOM+frontmatter-stripped —
            // go straight to the body so it isn't re-stripped.
            None => parse_flowchart(source).map(Document::Flowchart),
        };
    }
    Ok(Document::Flowchart(Graph::default()))
}

/// Parse a flowchart. For ER diagrams use [`parse_document`].
pub fn parse(source: &str) -> Result<Graph, ParseError> {
    // Lihat catatan BOM di parse_document — entry point ini publik
    // juga, jadi dapat perlakuan yang sama (BOM + frontmatter).
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let stripped = strip_frontmatter(source)?;
    parse_flowchart(&stripped)
}

/// The flowchart parse body, on a source that has ALREADY had its BOM
/// and frontmatter stripped. `parse` (public) and `parse_document`'s
/// flowchart arm both funnel through here so the strip happens exactly
/// once — otherwise a diagram BODY whose first line is itself `---`
/// would be mistaken for a second frontmatter block on the re-strip.
fn parse_flowchart(source: &str) -> Result<Graph, ParseError> {
    let mut g = Graph::default();
    // Accessibility statements are line-level in mermaid (an
    // `accTitle:` value runs to the end of the line, `;` included), so
    // they come out BEFORE the `;` statement split. Only lines after
    // the header qualify: mermaid parses them as statements.
    let header_line = source
        .lines()
        .position(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("%%")
        })
        .map_or(0, |i| i + 1);
    let (source, acc) = extract_acc(source, header_line, true, AccFlavor::Jison)?;
    let source: &str = &source;
    g.acc_title = acc.title;
    g.acc_descr = acc.descr;
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
    // box. A line like `subgraph --> B` (or `SUBGRAPH --> B` —
    // strip_keyword matches case-insensitively, so the as-written
    // HEAD token matters) is an edge, not a header, exactly when a
    // subgraph whose id equals that head exists. That's decidable up
    // front: real headers never start with an edge operator, so
    // their ids can be collected first without circularity.
    // Logical statements (physical lines split on top-level `;`).
    // Computed ONCE and shared by the pre-scan and the main pass so a
    // `;` can never make them classify a `subgraph` line differently.
    let stmts = statements(source)?;
    let header_sg_ids: std::collections::HashSet<String> = stmts
        .iter()
        .filter_map(|(_, line)| {
            let line: &str = line;
            let rest = strip_keyword(line, "subgraph")?;
            if edge_op_follows(rest) {
                return None;
            }
            parse_subgraph_header(rest.trim(), 0).ok().map(|(id, _)| id)
        })
        .collect();
    // Mermaid keywords are case-sensitive, so a case-variant head
    // (`SUBGRAPH --> B`) can never be the `subgraph` statement — it
    // routes to the edge path unconditionally. An exact-lowercase
    // head is an edge only when a subgraph with that literal id
    // exists (else it keeps the header diagnostics).
    let sg_line_is_edge = |line: &str, rest: &str| {
        edge_op_follows(rest) && (&line[..8] != "subgraph" || header_sg_ids.contains(&line[..8]))
    };
    let mut sub_ids: HashMap<String, usize> = HashMap::new();
    {
        let mut idx = 0usize;
        for (_, line) in &stmts {
            let line: &str = line;
            if let Some(rest) = strip_keyword(line, "subgraph") {
                if sg_line_is_edge(line, rest) {
                    continue;
                }
                if let Ok((id, _)) = parse_subgraph_header(rest.trim(), 0) {
                    sub_ids.entry(id).or_insert(idx);
                    idx += 1;
                }
            }
        }
    }

    let mut fm = FlowMeta::default();
    for (lineno, line) in &stmts {
        let (lineno, line): (usize, &str) = (*lineno, line);
        // `line` is trimmed and non-empty; blanks and full-line `%%`
        // comments were already dropped by `statements`.
        if !header_seen {
            header_seen = true;
            if let Some(t) = diagram_type(line) {
                // Every type `parse_document` can handle points at the
                // right entry point; only a genuinely unparsed header
                // (`gantt`) is reported as "not supported yet".
                let hint = if header_is_parsed(t) {
                    "this parser is flowchart-only — use parse_document() or render_svg()"
                        .to_string()
                } else {
                    format!("not supported yet (supported: {})", supported_types())
                };
                return Err(err(lineno, format!("diagram type '{}': {}", t, hint)));
            }
            let rest = strip_keyword(line, FLOWCHART_ELK_HEADER)
                .or_else(|| strip_keyword(line, "flowchart"))
                .or_else(|| strip_keyword(line, "graph"));
            if let Some(rest) = rest {
                g.direction = match rest.trim().to_uppercase().as_str() {
                    "" | "TD" | "TB" => Direction::TD,
                    "LR" => Direction::LR,
                    "RL" => Direction::RL,
                    "BT" => Direction::BT,
                    other => return Err(err(lineno, format!("unknown direction: '{}'", other))),
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
        // The head token AS WRITTEN decides (`Class --> B` —
        // strip_keyword matches case-insensitively, but mermaid
        // keywords are case-SENSITIVE, so a case-variant head can
        // never be the statement keyword and always routes to the
        // edge path). An exact-lowercase head diverts only when a
        // subgraph with that literal id exists. Keyword-named NODES
        // never need this path: the emitter writes them in full
        // `id["label"]` form.
        let kw_head_is_edge = |kw: &str, rest: &str| {
            edge_lead_follows(rest)
                && (&line[..kw.len()] != kw || sub_ids.contains_key(&line[..kw.len()]))
        };

        // Subgraph blocks: `subgraph id [Title]` ... `end`, nestable,
        // with an optional `direction XX` line inside.
        if let Some(rest) = strip_keyword(line, "subgraph") {
            if sg_line_is_edge(line, rest) {
                parse_statement(
                    &mut g,
                    line,
                    lineno,
                    &mut assigns,
                    &sub_stack,
                    &sub_ids,
                    &mut fm,
                )?;
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
        // `id@{ … }` on a subgraph or a user edge id is metadata for
        // it, never a node (mermaid flowDb `addVertex`: subgraph first,
        // then an edge whose id already exists). Anything else goes the
        // node path below.
        if let Some((id, body, after)) = meta_statement(line) {
            let on_sub = sub_ids.contains_key(id);
            if on_sub || fm.edges.contains_key(id) {
                if !after.trim().is_empty() {
                    return Err(err(
                        lineno,
                        format!(
                            "'{id}@{{…}}' must stand alone, found after it: '{}'",
                            after.trim()
                        ),
                    ));
                }
                let entries = shape_data(body, lineno)?;
                // Subgraph metadata (mermaid's `view: collapsed`) is
                // validated and dropped: nothing to draw with it yet.
                if let (false, Some(raw)) = (on_sub, fm.edges.get_mut(id)) {
                    raw.merge(&entries);
                }
                continue;
            }
        }
        if line == "end" && !sub_stack.is_empty() {
            sub_stack.pop();
            continue;
        }
        if let Some(rest) = strip_keyword(line, "direction") {
            if !(kw_head_is_edge("direction", rest)) {
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
            if kw_head_is_edge("classDef", rest) {
                parse_statement(
                    &mut g,
                    line,
                    lineno,
                    &mut assigns,
                    &sub_stack,
                    &sub_ids,
                    &mut fm,
                )?;
                continue;
            }
            let rest = rest.trim();
            let (names, props) = rest
                .split_once(char::is_whitespace)
                .ok_or_else(|| err(lineno, "classDef needs a name and properties".to_string()))?;
            let st = parse_props(props.trim(), lineno)?;
            for name in names.split(',').filter(|n| !n.is_empty()) {
                class_defs.insert(name.to_string(), st.clone());
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "class") {
            if kw_head_is_edge("class", rest) {
                parse_statement(
                    &mut g,
                    line,
                    lineno,
                    &mut assigns,
                    &sub_stack,
                    &sub_ids,
                    &mut fm,
                )?;
                continue;
            }
            let rest = rest.trim();
            // The LAST token is the class name; everything before is
            // the id list, so `class A, B hot` (space after comma)
            // styles both A and B.
            let (ids, name) = rest
                .rsplit_once(char::is_whitespace)
                .ok_or_else(|| err(lineno, "class needs node ids and a class name".to_string()))?;
            for id in ids.split(',').filter(|i| !i.trim().is_empty()) {
                let id = id.trim();
                // Styling a SUBGRAPH box by id is legal mermaid;
                // flowmaid doesn't paint subgraph styles yet, so the
                // assignment is accepted and dropped — minting a
                // node here would collide with the subgraph id. The
                // subgraph check runs FIRST: subgraph ids are
                // free-form (`my-group`), node ids are word-like.
                if sub_ids.contains_key(id) {
                    continue;
                }
                // An edge id (`A e1@--> B` … `class e1 animate`) styles
                // the edge in mermaid; flowmaid doesn't paint edge
                // classes, and must not mint a stray node named so.
                if g.node_index(id).is_none() && fm.edges.contains_key(id) {
                    continue;
                }
                if id.contains(char::is_whitespace) {
                    return Err(err(
                        lineno,
                        format!(
                            "class takes a comma-separated id list then ONE class name; \
                             unexpected extra token in: '{id}'"
                        ),
                    ));
                }
                let n = g.ensure_node(check_stmt_id(id, "class", lineno)?, None, None);
                assigns.push((n, name.trim().to_string()));
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "style") {
            if kw_head_is_edge("style", rest) {
                parse_statement(
                    &mut g,
                    line,
                    lineno,
                    &mut assigns,
                    &sub_stack,
                    &sub_ids,
                    &mut fm,
                )?;
                continue;
            }
            let rest = rest.trim();
            let (id, props) = rest
                .split_once(char::is_whitespace)
                .ok_or_else(|| err(lineno, "style needs a node id and properties".to_string()))?;
            let id = id.trim();
            // Subgraph-box styling: accepted, not painted (see the
            // `class` branch note) — never mint a colliding node.
            // Checked BEFORE the word-like id rule (subgraph ids are
            // free-form), and the props still get validated.
            if sub_ids.contains_key(id) || (g.node_index(id).is_none() && fm.edges.contains_key(id))
            {
                parse_props(props.trim(), lineno)?;
            } else {
                let n = g.ensure_node(check_stmt_id(id, "style", lineno)?, None, None);
                styles.push((n, parse_props(props.trim(), lineno)?));
            }
            continue;
        }
        // `linkStyle`: exact-case head only — mermaid keywords are
        // case-sensitive — and `linkStyle --> B` stays an EDGE from a
        // node named linkStyle.
        if let Some(rest) = strip_keyword(line, LINK_STYLE_KW) {
            if &line[..LINK_STYLE_KW.len()] == LINK_STYLE_KW && !edge_lead_follows(rest) {
                parse_link_style(rest, lineno, &mut fm.links)?;
                continue;
            }
        }
        // Known mermaid statements flowmaid doesn't implement get a
        // targeted message instead of a baffling edge-operator error.
        // Exact-case head only — mermaid keywords are case-sensitive,
        // so `Click` is a plain node id — and `click --> B` stays an
        // EDGE from a node named click.
        // `click`: exact-case head only (`Click` is a node id), and
        // `click --> B` stays an EDGE from a node named click.
        if let Some(rest) = strip_keyword(line, CLICK_KW) {
            if &line[..CLICK_KW.len()] == CLICK_KW && !edge_lead_follows(rest) {
                apply_click(&mut g, parse_click(rest, lineno)?, &mut fm);
                continue;
            }
        }

        parse_statement(
            &mut g,
            line,
            lineno,
            &mut assigns,
            &sub_stack,
            &sub_ids,
            &mut fm,
        )?;
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

    // Edge metadata resolves now that every `id@{ … }` is merged.
    let resolve = |meta: &mut EdgeMeta| {
        if let Some(raw) = meta.id.as_ref().and_then(|id| fm.edges.get(id)) {
            (meta.animation, meta.curve) = raw.resolve();
        }
    };
    g.edges.iter_mut().for_each(|e| resolve(&mut e.meta));
    g.sub_edges.iter_mut().for_each(|e| resolve(&mut e.meta));
    // `click` tooltips and callbacks reach every node that exists by
    // now; links with nothing left on them go.
    for (map, tip) in [(&fm.click_tips, true), (&fm.click_callbacks, false)] {
        for (id, v) in map {
            if let Some(n) = g.node_index(id) {
                let l = g.links.entry(n).or_default();
                let slot = if tip { &mut l.tooltip } else { &mut l.callback };
                *slot = Some(v.clone());
            }
        }
    }
    g.links.retain(|_, l| !l.is_empty());
    // `linkStyle` resolves after it: `default` reaches every link,
    // including those defined after it, and `@{ curve }` wins.
    for (k, r) in fm.links.order.iter().enumerate() {
        let meta = match *r {
            LinkRef::Edge(i) => &mut g.edges[i].meta,
            LinkRef::Sub(i) => &mut g.sub_edges[i].meta,
        };
        fm.links.resolve(k, meta);
    }

    // Resolve styling: classDef layers first, explicit `style`
    // lines win. Unknown class names are ignored, mermaid-style.
    // Borderless mermaid shapes first, so `style` / `classDef` win.
    for &n in &fm.borderless {
        let st = &mut g.nodes[n].style;
        st.fill = Some(BORDERLESS_PAINT.to_string());
        st.stroke = Some(BORDERLESS_PAINT.to_string());
    }
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
/// Unknown properties are ignored (mermaid tolerates them too). An item
/// without `:` after a `property:value` one is the tail of a value that
/// held a comma — mermaid's `classDef`/`style` split on every comma, so
/// `stroke-dasharray: 9,5` leaves a bare `5` it keeps as a no-op
/// declaration — and is ignored, as [`parse_link_props`] does. A bare
/// item with no property before it is still an error.
fn parse_props(s: &str, lineno: usize) -> Result<NodeStyle, ParseError> {
    let mut st = NodeStyle::default();
    let items = split_props(s, lineno)?;
    let mut after_prop = false;
    for item in items {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let Some((k, v)) = item.split_once(':') else {
            if after_prop {
                continue;
            }
            return Err(err(
                lineno,
                format!("expected 'property:value', got '{}'", item),
            ));
        };
        after_prop = true;
        let v = v.trim();
        match k.trim() {
            "fill" => st.fill = Some(v.to_string()),
            "stroke" => st.stroke = Some(v.to_string()),
            "color" => st.color = Some(v.to_string()),
            "stroke-width" => st.stroke_width = Some(parse_stroke_width(v, lineno)?),
            _ => {}
        }
    }
    Ok(st)
}

/// How flowmaid treats a Mermaid diagram header it recognises.
#[derive(Clone, Copy, PartialEq)]
enum HeaderSupport {
    /// A parser exists, and the token is named in the "supported" list.
    Parsed,
    /// A parser exists, but the token is a spelling variant of one
    /// already listed (`stateDiagram` for `stateDiagram-v2`), so it is
    /// matched without being advertised twice.
    Alias,
    /// Recognised ONLY so the header fails with a clear message instead
    /// of being parsed as a flowchart node.
    Unsupported,
}

/// Every Mermaid diagram header flowmaid recognises, and how each one
/// is treated.
///
/// This table is the single source of truth: [`diagram_type`] matches
/// against it, BOTH parser entry points build their "supported: ..."
/// list from it, and the CLI `--help` quotes the same list — so a
/// diagram type is declared in exactly one place.
///
/// Order does not affect matching: [`diagram_type`] requires a
/// whitespace-or-end-of-line boundary after the token, so `stateDiagram`
/// can never shadow `stateDiagram-v2` however the two are ordered. Order
/// DOES fix the reading order of the user-facing list, which is why the
/// `Parsed` entries come first, in the order the README introduces them.
const DIAGRAM_HEADERS: &[(&str, HeaderSupport)] = &[
    // Parsed — listed, in README order.
    ("erDiagram", HeaderSupport::Parsed),
    ("classDiagram", HeaderSupport::Parsed),
    ("sequenceDiagram", HeaderSupport::Parsed),
    ("pie", HeaderSupport::Parsed),
    ("stateDiagram-v2", HeaderSupport::Parsed),
    ("mindmap", HeaderSupport::Parsed),
    ("journey", HeaderSupport::Parsed),
    ("gitGraph", HeaderSupport::Parsed),
    ("architecture-beta", HeaderSupport::Parsed),
    ("timeline", HeaderSupport::Parsed),
    ("sankey-beta", HeaderSupport::Parsed),
    // Parsed, but a spelling variant of an entry already listed.
    ("stateDiagram", HeaderSupport::Alias),
    ("architecture", HeaderSupport::Alias),
    ("sankey", HeaderSupport::Alias),
    // Recognised only so they fail with a clear message rather than
    // being parsed as flowchart nodes.
    ("swimlane-beta", HeaderSupport::Unsupported),
    ("requirementDiagram", HeaderSupport::Unsupported),
    ("quadrantChart", HeaderSupport::Unsupported),
    ("xychart-beta", HeaderSupport::Unsupported),
    ("block-beta", HeaderSupport::Unsupported),
    ("packet-beta", HeaderSupport::Unsupported),
    ("packet", HeaderSupport::Unsupported),
    ("radar-beta", HeaderSupport::Unsupported),
    ("treemap-beta", HeaderSupport::Unsupported),
    ("treemap", HeaderSupport::Unsupported),
    ("kanban", HeaderSupport::Unsupported),
    ("zenuml", HeaderSupport::Unsupported),
    ("C4Context", HeaderSupport::Unsupported),
    ("C4Container", HeaderSupport::Unsupported),
    ("C4Component", HeaderSupport::Unsupported),
    ("C4Dynamic", HeaderSupport::Unsupported),
    ("C4Deployment", HeaderSupport::Unsupported),
    ("gantt", HeaderSupport::Unsupported),
];

/// Recognise a known Mermaid diagram-type header other than
/// flowchart/graph, so we can fail with a clear message instead of
/// parsing the header as a node.
fn diagram_type(line: &str) -> Option<&'static str> {
    DIAGRAM_HEADERS.iter().map(|&(t, _)| t).find(|t| {
        line.get(..t.len()) == Some(*t)
            && line[t.len()..]
                .chars()
                .next()
                // Mermaid's gitGraph grammar also takes `gitGraph:`.
                .map_or(true, |c| {
                    c.is_whitespace() || (*t == "gitGraph" && c == ':')
                })
    })
}

/// Does flowmaid have a parser for this recognised header? Drives the
/// hint in [`parse`]: a parsed type means "wrong entry point", while an
/// unparsed one really is "not supported yet".
fn header_is_parsed(t: &str) -> bool {
    DIAGRAM_HEADERS
        .iter()
        .any(|&(name, sup)| name == t && sup != HeaderSupport::Unsupported)
}

/// The "supported: ..." list both parser entry points quote, built
/// from [`DIAGRAM_HEADERS`] so it stays in step with the dispatch.
/// Public so the CLI `--help` can quote the very same list.
pub fn supported_types() -> String {
    let mut out = String::from("flowchart, graph");
    for &(name, sup) in DIAGRAM_HEADERS {
        if sup == HeaderSupport::Parsed {
            out.push_str(", ");
            out.push_str(name);
        }
    }
    out
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
    // Bare markdown title (`subgraph "`**Two**`"`): mermaid's id is the
    // markdown text; flowmaid keeps it even with spaces, as it does for
    // a bare multi-word title.
    if let Some(inner) = markdown::md_quoted(rest) {
        return Ok((inner.trim().to_string(), md_label(inner, lineno, false)?));
    }
    if let Some(bi) = rest.find('[') {
        let id = rest[..bi].trim();
        let inner = rest[bi + 1..]
            .strip_suffix(']')
            .ok_or_else(|| err(lineno, "subgraph title '[' is never closed".to_string()))?;
        if id.is_empty() {
            return Err(err(
                lineno,
                "subgraph title needs an id before '['".to_string(),
            ));
        }
        // A markdown string is lowered to one line; a plain title keeps
        // its `<br/>` breaks as `\n` so the frame draws one row each.
        let title = match markdown::md_quoted(inner) {
            Some(md) => md_label(md, lineno, false)?,
            None => clean_label(inner),
        };
        return Ok((id.to_string(), title));
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
    fm: &mut FlowMeta,
) -> Result<Vec<End>, ParseError> {
    let mut nodes = vec![parse_node(cur, g, lineno, assigns, subs, sub_ids, fm)?];
    loop {
        let save = cur.pos;
        cur.skip_ws();
        if cur.eat("&") {
            cur.skip_ws();
            nodes.push(parse_node(cur, g, lineno, assigns, subs, sub_ids, fm)?);
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
    fm: &mut FlowMeta,
) -> Result<(), ParseError> {
    let mut cur = Cur::new(line);
    let mut prevs = parse_node_list(&mut cur, g, lineno, assigns, subs, sub_ids, fm)?;
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
            prevs = parse_node_list(&mut cur, g, lineno, assigns, subs, sub_ids, fm)?;
            continue;
        }
        // Mermaid's optional edge id before the operator (`e1@-->`).
        let link_id = parse_link_id(&mut cur);
        // Inline-labelled operators (`-- text -->`) carry their own
        // label; plain operators may take a `|text|` label after.
        let (op, mut label) = match parse_edge_inline_label(&mut cur, lineno)? {
            Some((k, e, l)) => ((k, e), Some(l)),
            None => (
                parse_edge_op(&mut cur).ok_or_else(|| {
                    err(
                        lineno,
                        format!("unknown edge operator near: '{}'", cur.rest()),
                    )
                })?,
                None,
            ),
        };
        let (kind, ends) = op;
        cur.skip_ws();
        if label.is_none() && cur.eat("|") {
            // A markdown string may hold `|`; it ends only at `` `" ``.
            let save = cur.pos;
            cur.skip_ws();
            if cur.eat(markdown::MD_OPEN) {
                let inner = cur
                    .take_until(markdown::MD_CLOSE)
                    .ok_or_else(|| err(lineno, "unclosed markdown string".to_string()))?;
                cur.skip_ws();
                if !cur.eat("|") {
                    return Err(err(
                        lineno,
                        "edge label opened with '|' but never closed".to_string(),
                    ));
                }
                label = Some(md_label(&inner, lineno, false)?);
            } else {
                cur.pos = save;
                let l = cur.take_until("|").ok_or_else(|| {
                    err(
                        lineno,
                        "edge label opened with '|' but never closed".to_string(),
                    )
                })?;
                label = Some(clean_label_1line(&l));
            }
        }
        cur.skip_ws();
        let nexts = parse_node_list(&mut cur, g, lineno, assigns, subs, sub_ids, fm)?;
        // The id goes to ONE pair only — the last source and the first
        // target, as in mermaid (`A & B e1@--> C & D`: B→C) — and only
        // while no other edge holds it; a repeat id leaves the edge
        // without one, silently, as mermaid's auto-id fallback does.
        let id_pair = (*prevs.last().expect("non-empty"), nexts[0]);
        let mut link_id = link_id.filter(|id| !fm.edges.contains_key(id));
        // Fan-out: every prev connects to every next. Node→node
        // edges go to `edges`; anything touching a subgraph to
        // `sub_edges` (drawn against the cluster box).
        for &a in &prevs {
            for &b in &nexts {
                let meta = EdgeMeta {
                    id: if (a, b) == id_pair {
                        link_id.take()
                    } else {
                        None
                    },
                    ..EdgeMeta::default()
                };
                if let Some(id) = &meta.id {
                    fm.edges.insert(id.clone(), RawEdgeMeta::default());
                }
                match (a, b) {
                    (End::Node(x), End::Node(y)) => {
                        g.add_edge_with_ends(x, y, label.clone(), kind, ends);
                        if let Some(e) = g.edges.last_mut() {
                            e.meta = meta;
                        }
                        fm.links.push(LinkRef::Edge(g.edges.len() - 1));
                    }
                    _ => {
                        g.sub_edges.push(SubEdge {
                            from: a,
                            to: b,
                            label: label.clone(),
                            kind,
                            ends,
                            meta,
                        });
                        fm.links.push(LinkRef::Sub(g.sub_edges.len() - 1));
                    }
                }
            }
        }
        prevs = nexts;
    }
    Ok(())
}

/// Mermaid's inline edge labels: `-- text -->`, `-- text ---`,
/// `-. text .->`, `-. text .-`, `== text ==>`, `== text ===`, the
/// marker closers `-- text --x` / `--o` (and `.-x`, `==x`, …), and
/// the two-way forms `<-- text -->`, `x-- text --x`, `o-- text --o`.
/// Returns None when the cursor isn't at an inline-label operator
/// (plain operators like `-->` or `--x` fall through to
/// `parse_edge_op`).
fn parse_edge_inline_label(
    cur: &mut Cur<'_>,
    lineno: usize,
) -> Result<Option<(EdgeKind, EdgeEnds, String)>, ParseError> {
    let rest = cur.rest();
    // Optional start marker, mermaid's `[xo<]?` before the opener.
    let (lead, body) = match rest.chars().next() {
        Some(c @ ('<' | 'x' | 'o')) => (Some(c), &rest[1..]),
        _ => (None, rest),
    };
    for open in ["-.", "==", "--"] {
        if !body.starts_with(open) {
            continue;
        }
        // Only label mode when the opener is followed by label text,
        // not by more operator characters (`-->`, `---`, `-.-`, ...).
        // `--x` / `--o` / `==x` / `==o` are complete marker operators
        // — mermaid's lexer tries its LINK rule before START_LINK —
        // so they never open a label either.
        let after = &body[open.len()..];
        let Some(c0) = after.chars().next() else {
            continue;
        };
        if matches!(c0, '-' | '>' | '.' | '=') || (open != "-." && matches!(c0, 'x' | 'o')) {
            return Ok(None);
        }
        // The earliest closer wins.
        let Some(InlineCloser {
            at: i,
            len,
            kind,
            end,
            lead: close_lead,
        }) = find_inline_closer(open, after)
        else {
            return Err(err(
                lineno,
                format!(
                    "inline edge label after '{}{open}' is never closed",
                    lead.map(String::from).unwrap_or_default()
                ),
            ));
        };
        // Mermaid accepts a start marker on a labelled edge only when
        // it mirrors the closer (`<-- t -->`, `x-- t --x`); a lone or
        // mismatched one is an error there too. A closer that mirrors
        // itself (`o--o`, `<-->`) makes the edge two-way, unless the
        // opener already has a start character (also an error there).
        let resolved_end = EdgeEnds { start: None, end }.end_marker(kind);
        let two_way_closer = close_lead.is_some_and(|c| lead_marker(c) == resolved_end);
        let start = match lead {
            None if two_way_closer => resolved_end,
            None => None,
            Some(c) if !two_way_closer && lead_marker(c) == resolved_end => resolved_end,
            Some(c) => {
                return Err(err(
                    lineno,
                    format!(
                        "edge start '{c}' needs the matching end marker \
                         (`<-- text -->`, `x-- text --x`, `o-- text --o`)"
                    ),
                ))
            }
        };
        let label = label_1line(&after[..i], lineno)?;
        cur.pos += (rest.len() - body.len()) + open.len() + i + len;
        return Ok(Some((kind, EdgeEnds { start, end }, label)));
    }
    Ok(None)
}

/// The marker a `[xo<]` start character asks for.
fn lead_marker(c: char) -> Option<Marker> {
    match c {
        '<' => Some(Marker::Arrow),
        'x' => Some(Marker::Cross),
        'o' => Some(Marker::Circle),
        _ => None,
    }
}

/// The marker an `x` / `o` operator tail stands for.
fn tail_marker(c: Option<char>) -> Option<Marker> {
    match c {
        Some('x') => Some(Marker::Cross),
        Some('o') => Some(Marker::Circle),
        _ => None,
    }
}

/// A closer found in inline label text: where it starts, how long it
/// is, the line kind, its explicit end marker, and the `[xo<]` start
/// character glued to its front (if any).
struct InlineCloser {
    at: usize,
    len: usize,
    kind: EdgeKind,
    end: Option<Marker>,
    lead: Option<char>,
}

/// The earliest closer of an inline label opened by `open` in `after`
/// (the text right after the opener). Like mermaid's lexer, which
/// tries its LINK rule `\s*[xo<]?…` before each edge-text character,
/// a closer may carry a `[xo<]` start character taken from the label
/// (`-- go--on -->` closes at `o--o`, label `g`). A `"…"` span is
/// skipped whole, as mermaid's string state does, so a quoted label
/// may hold closer-looking text (`-- "use --xml" -->`). An unmatched
/// `"` is plain text.
fn find_inline_closer(open: &str, after: &str) -> Option<InlineCloser> {
    let mut i = 0;
    while i < after.len() {
        let s = &after[i..];
        if let Some(q) = s.strip_prefix('"').and_then(|t| t.find('"')) {
            i += q + 2;
            continue;
        }
        let lead = s.chars().next().filter(|c| matches!(c, 'x' | 'o' | '<'));
        let found = lead
            .and_then(|c| inline_closer(open, &s[c.len_utf8()..]).map(|f| (f, Some(c))))
            .or_else(|| inline_closer(open, s).map(|f| (f, None)));
        if let Some(((len, kind, end), lead)) = found {
            let lead_len = lead.map_or(0, char::len_utf8);
            return Some(InlineCloser {
                at: i,
                len: lead_len + len,
                kind,
                end,
                lead,
            });
        }
        i += s.chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// Whether `s` starts with the closer of an inline label opened by
/// `open`; returns (closer length, kind, explicit end marker). The
/// solid and thick closers are mermaid's `--+[-xo>]` / `==+[=xo>]`
/// (any length), the dotted one `-?.+-[xo>]?`.
fn inline_closer(open: &str, s: &str) -> Option<(usize, EdgeKind, Option<Marker>)> {
    match open {
        "-." => {
            let dash = usize::from(s.starts_with('-'));
            let dots = s[dash..].chars().take_while(|&c| c == '.').count();
            if dots == 0 {
                return None;
            }
            let body = dash + dots;
            let tail = s[body..].strip_prefix('-')?;
            let c = tail.chars().next();
            if c == Some('>') {
                Some((body + 2, EdgeKind::Dotted, None))
            } else if let Some(m) = tail_marker(c) {
                Some((body + 2, EdgeKind::DottedOpen, Some(m)))
            } else {
                Some((body + 1, EdgeKind::DottedOpen, None))
            }
        }
        _ => {
            let ch = open.chars().next()?;
            let (arrow, line) = if ch == '=' {
                (EdgeKind::Thick, EdgeKind::ThickOpen)
            } else {
                (EdgeKind::Arrow, EdgeKind::Open)
            };
            let n = s.chars().take_while(|&c| c == ch).count();
            if n < 2 {
                return None;
            }
            let c = s[n..].chars().next();
            if c == Some('>') {
                Some((n + 1, arrow, None))
            } else if let Some(m) = tail_marker(c) {
                Some((n + 1, line, Some(m)))
            } else if n >= 3 {
                Some((n, line, None))
            } else {
                None
            }
        }
    }
}

fn parse_node(
    cur: &mut Cur<'_>,
    g: &mut Graph,
    lineno: usize,
    assigns: &mut Vec<(usize, String)>,
    subs: &[usize],
    sub_ids: &HashMap<String, usize>,
    fm: &mut FlowMeta,
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
    let parsed: Option<(Shape, (String, bool))> = if cur.eat("(((") {
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
        // `[/x/]` lean-right, `[/x\]` trapezoid (drawn as its
        // `trap-b` shape-data fallback).
        let closer = slant_closer(cur.rest(), LEAN_R_CLOSE);
        Some((Shape::Parallelogram, close(cur, closer, lineno)?))
    } else if cur.eat("[\\") {
        // `[\x\]` lean-left, `[\x/]` inverted trapezoid (`trap-t`).
        let closer = slant_closer(cur.rest(), LEAN_L_CLOSE);
        Some((Shape::ParallelogramAlt, close(cur, closer, lineno)?))
    } else if cur.eat(">") {
        // `>x]` mermaid's asymmetric `odd` shape, drawn as its
        // shape-data fallback.
        Some((Shape::Rect, close(cur, "]", lineno)?))
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
        Some((s, (l, true))) => (Some(s), Some(md_label(&l, lineno, true)?)),
        Some((s, (l, false))) => (Some(s), Some(clean_label(&l))),
        None => (None, None),
    };
    // An id that names a declared subgraph (pre-scanned, so
    // forward references work) is an edge endpoint on the whole
    // cluster (`CF --> VPC`). A subgraph id wins even if a shape
    // was written (`VPC[x]`) — you can't have a node and a
    // subgraph sharing an id — so no duplicate node is created.
    if g.node_index(&id).is_none() {
        if let Some(&si) = sub_ids.get(&id) {
            // Subgraph metadata (`s1@{ view: collapsed }`): validated,
            // nothing to draw with it yet.
            if cur.eat("@{") {
                node_meta_body(cur, lineno)?;
            }
            return Ok(End::Sub(si));
        }
    }
    let n = g.ensure_node(&id, label, shape);
    // A bracket shape replaces whatever an earlier `@{ shape: … }` set,
    // borderless text included.
    if shape.is_some() {
        fm.borderless.remove(&n);
    }
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
    // Mermaid v11 node metadata, after the bracket shape and `:::class`
    // and glued to them (`A[x]:::c@{ shape: cyl }`); it overrides both
    // the shape and the label.
    if cur.eat("@{") {
        apply_node_meta(g, n, &node_meta_body(cur, lineno)?, fm, lineno)?;
    }
    Ok(End::Node(n))
}

/// Which closer ends the slanted shape (`[/…` / `[\…`) whose label
/// starts `rest`: the earlier of its `own` closer and the other slant
/// (`/]` vs `\]`) after any quoted label, since mermaid lexes the first
/// one it meets. Defaults to `own`, so an unclosed label reports it.
fn slant_closer(rest: &str, own: &'static str) -> &'static str {
    let other = if own == LEAN_R_CLOSE {
        LEAN_L_CLOSE
    } else {
        LEAN_R_CLOSE
    };
    let from = match rest.strip_prefix('"') {
        Some(q) => q.find('"').map_or(rest.len(), |e| e + 2),
        None => 0,
    };
    let tail = &rest[from..];
    match (tail.find(own), tail.find(other)) {
        (Some(a), Some(b)) if b < a => other,
        (None, Some(_)) => other,
        _ => own,
    }
}

/// The `/]` and `\]` closers of the slanted bracket shapes.
const LEAN_R_CLOSE: &str = "/]";
const LEAN_L_CLOSE: &str = "\\]";

/// A bracket label's text up to `closer`, and whether it is a
/// markdown string (returned without its delimiters, still raw).
fn close(cur: &mut Cur<'_>, closer: &str, lineno: usize) -> Result<(String, bool), ParseError> {
    // Markdown string: `A["`**x**`"]` — only `` `" `` ends it.
    if cur.eat(markdown::MD_OPEN) {
        let inner = cur
            .take_until(markdown::MD_CLOSE)
            .ok_or_else(|| err(lineno, "unclosed markdown string".to_string()))?;
        cur.skip_ws();
        if !cur.eat(closer) {
            return Err(err(lineno, format!("closing '{}' not found", closer)));
        }
        return Ok((inner, true));
    }
    close_plain(cur, closer, lineno).map(|l| (l, false))
}

fn close_plain(cur: &mut Cur<'_>, closer: &str, lineno: usize) -> Result<String, ParseError> {
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

/// Byte length of the edge operator (`-->`, `-.->`, `==x`, …) or the
/// whole inline-label edge (`-- text -->`, `-. text .->`,
/// `== text ==>`) that starts `s`, or `None` when `s` does not start
/// one. An inline label that never closes runs to the end of `s`, so
/// its text is still label text (the edge parser reports the error).
/// Lets the shape-data scanner step over label text the way mermaid's
/// `edgeText` lexer state does.
pub(crate) fn edge_span(s: &str) -> Option<usize> {
    let mut cur = Cur::new(s);
    if parse_edge_op(&mut cur).is_some() {
        return Some(cur.pos);
    }
    let open = ["--", "-.", "=="].into_iter().find(|o| s.starts_with(o))?;
    let after = &s[open.len()..];
    let c0 = after.chars().next()?;
    if matches!(c0, '-' | '>' | '.' | '=') || (open != "-." && matches!(c0, 'x' | 'o')) {
        return None;
    }
    Some(find_inline_closer(open, after).map_or(s.len(), |c| open.len() + c.at + c.len))
}

/// Like [`edge_op_follows`] but also recognizing the inline-label
/// edge forms (`-- text -->`, `-. text .->`, `== text ==>` and their
/// open-line closers). The opener alone is ambiguous with statement
/// arguments like `--x`, so the closing operator must appear later
/// on the line, with a target after it (a line ENDING in the closer
/// is a bare-title subgraph header, not an edge). A complete operator
/// likewise needs a target after it. Label text is
/// unrestricted — `-- (note) -->` and `-- open: docs -->` are real
/// edges — mirroring what the inline-label edge parser consumes.
/// Used by the statement-keyword gates only — the `subgraph` header
/// branch sticks to complete operators so 0.19-era headers titled
/// `-- Phase 1 ---` keep parsing as headers.
fn edge_lead_follows(rest: &str) -> bool {
    let r = rest.trim_start();
    if r.starts_with('&') {
        return true;
    }
    // A complete operator counts only with a target after it: a bare
    // `click x--x` is a malformed statement, not an edge from `click`.
    let mut c = Cur::new(r);
    if parse_edge_op(&mut c).is_some() && !c.rest().trim().is_empty() {
        return true;
    }
    let r = r
        .strip_prefix(['<', 'x', 'o'])
        .filter(|b| ["--", "-.", "=="].iter().any(|o| b.starts_with(o)))
        .unwrap_or(r);
    let Some(open) = ["--", "-.", "=="].into_iter().find(|o| r.starts_with(o)) else {
        return false;
    };
    // Earliest closer wins (that's what the edge parser would eat).
    let end = find_inline_closer(open, &r[2..]).map(|c| 2 + c.at + c.len);
    end.is_some_and(|end| !r[end..].trim().is_empty())
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
/// extra length (`--->`, `-..->`, `===>`). Marker tails (`--x`,
/// `-.-o`, `==x`) and mermaid's `[xo<]` start character (`<-->`,
/// `x--x`, `o-.-o`) are part of the operator. Like mermaid, a start
/// character only counts when it mirrors the end (`<` with an
/// arrowhead, `x` with `x`, `o` with `o`); otherwise it is eaten and
/// ignored (`<---` is a plain line).
///
/// The tail is lexical, exactly as in mermaid: `A---oB` is a circle
/// edge to `B` and `A--xray` a cross edge to `ray`. Write a space
/// (`A --- oB`) to target a node whose id starts with `o`/`x`.
fn parse_edge_op(cur: &mut Cur<'_>) -> Option<(EdgeKind, EdgeEnds)> {
    let rest = cur.rest();

    // Invisible link: ~~~ (layout-only, never drawn).
    if rest.starts_with("~~~") {
        let tildes = rest.chars().take_while(|&c| c == '~').count();
        cur.pos += tildes;
        return Some((EdgeKind::Invisible, EdgeEnds::default()));
    }

    let (lead, body) = match rest.chars().next() {
        Some(c @ ('<' | 'x' | 'o')) => (Some(c), &rest[1..]),
        _ => (None, rest),
    };
    let (len, kind, end) = edge_op_body(body)?;
    let resolved_end = EdgeEnds { start: None, end }.end_marker(kind);
    let start = lead
        .and_then(lead_marker)
        .filter(|&m| Some(m) == resolved_end);
    cur.pos += (rest.len() - body.len()) + len;
    Some((kind, EdgeEnds { start, end }))
}

/// The operator proper (after any start character): (length, kind,
/// explicit end marker).
fn edge_op_body(rest: &str) -> Option<(usize, EdgeKind, Option<Marker>)> {
    // Dotted: -.-> / -..-> (arrow), -.-x / -.-o (marker), -.- (open)
    if let Some(after) = rest.strip_prefix("-.") {
        let dots = after.chars().take_while(|&c| c == '.').count();
        let tail = after[dots..].strip_prefix('-')?;
        let c = tail.chars().next();
        if c == Some('>') {
            return Some((2 + dots + 2, EdgeKind::Dotted, None));
        }
        if let Some(m) = tail_marker(c) {
            return Some((2 + dots + 2, EdgeKind::DottedOpen, Some(m)));
        }
        return Some((2 + dots + 1, EdgeKind::DottedOpen, None));
    }

    // Thick: ==> / ===> (arrow), ==x / ==o (marker), === (open)
    if rest.starts_with("==") {
        let eqs = rest.chars().take_while(|&c| c == '=').count();
        let c = rest[eqs..].chars().next();
        if c == Some('>') {
            return Some((eqs + 1, EdgeKind::Thick, None));
        }
        if let Some(m) = tail_marker(c) {
            return Some((eqs + 1, EdgeKind::ThickOpen, Some(m)));
        }
        if eqs >= 3 {
            return Some((eqs, EdgeKind::ThickOpen, None));
        }
        return None;
    }

    // Regular arrow (-->), marker (--x / --o) or plain line (---)
    if rest.starts_with('-') {
        let dashes = rest.chars().take_while(|&c| c == '-').count();
        let c = rest[dashes..].chars().next();
        if dashes >= 2 && c == Some('>') {
            return Some((dashes + 1, EdgeKind::Arrow, None));
        }
        if dashes >= 2 {
            if let Some(m) = tail_marker(c) {
                return Some((dashes + 1, EdgeKind::Open, Some(m)));
            }
        }
        if dashes >= 3 {
            return Some((dashes, EdgeKind::Open, None));
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
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Jison)?;
    let source: &str = &source;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;
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
            if er_block_line(&mut d, ent, line, lineno)? {
                open = None;
            }
            continue;
        }

        if line == "}" {
            return Err(err(lineno, "'}' without an open entity block".to_string()));
        }
        // `direction TB|BT|LR|RL`. Only a lone word after the keyword:
        // `direction ||--o{ B` relates an entity named `direction`.
        if let Some(rest) = strip_keyword(line, "direction") {
            let word = rest.trim();
            if !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()) {
                d.direction = parse_direction(word, lineno)?;
                continue;
            }
        }
        if let Some(ent) = parse_er_statement(&mut d, line, lineno)? {
            open = Some((ent, lineno));
        }
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

/// One line of an open entity block (or what follows the `{` of the
/// line that opened it): attribute rows, then optionally the closing
/// `}` — mermaid tokenises a block, so `A { int id PK }` fits on one
/// line and several attributes may share one. Returns whether the
/// block closed.
fn er_block_line(
    d: &mut ErDiagram,
    ent: usize,
    line: &str,
    lineno: usize,
) -> Result<bool, ParseError> {
    let (body, closed) = match find_unquoted(line, "}") {
        Some(p) => {
            let after = line[p + 1..].trim();
            if !after.is_empty() {
                return Err(err(
                    lineno,
                    format!("unexpected text after '}}': '{}'", after),
                ));
            }
            (line[..p].trim(), true)
        }
        None => (line, false),
    };
    if !body.is_empty() {
        let attrs = parse_attrs(body, lineno)?;
        d.entities[ent].attrs.extend(attrs);
    }
    Ok(closed)
}

/// One top-level ER line: an entity declaration (`Name`, `Name[Alias]`),
/// the start of an entity block (`Name [Alias] {`, possibly with rows
/// and even its `}` on the same line), or a relationship
/// `A <card><line><card> B : "label"`. Returns the entity whose block
/// the line left open.
fn parse_er_statement(
    d: &mut ErDiagram,
    line: &str,
    lineno: usize,
) -> Result<Option<usize>, ParseError> {
    let mut cur = Cur::new(line);
    let (a, alias_a) = parse_er_entity(&mut cur, lineno)?;
    cur.skip_ws();
    if cur.at_end() {
        er_declare(d, &a, alias_a);
        return Ok(None);
    }
    if cur.eat("{") {
        let ent = er_declare(d, &a, alias_a);
        let rest = cur.rest().trim();
        let closed = !rest.is_empty() && er_block_line(d, ent, rest, lineno)?;
        return Ok((!closed).then_some(ent));
    }
    let (card_from, identifying, card_to) = parse_rel_op(&mut cur).ok_or_else(|| {
        err(
            lineno,
            format!("unknown relationship operator near: '{}'", cur.rest()),
        )
    })?;
    cur.skip_ws();
    let (b, alias_b) = parse_er_entity(&mut cur, lineno)?;
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
    let from = er_declare(d, &a, alias_a);
    let to = er_declare(d, &b, alias_b);
    d.relations.push(Relation {
        from,
        to,
        card_from,
        card_to,
        identifying,
        label,
    });
    Ok(None)
}

/// [`ErDiagram::ensure_entity`] plus an alias. As in mermaid, the first
/// alias an entity is given sticks.
fn er_declare(d: &mut ErDiagram, name: &str, alias: Option<String>) -> usize {
    let i = d.ensure_entity(name);
    let e = &mut d.entities[i];
    if e.alias.is_none() {
        e.alias = alias;
    }
    i
}

/// An entity reference: a name (word characters and `-`, or a
/// `"quoted name"` as in mermaid) and an optional display alias in
/// brackets (`p[Person]`, `c["Customer Account"]`). Mermaid accepts the
/// alias only where an entity is declared; flowmaid also takes it in a
/// relationship.
fn parse_er_entity(
    cur: &mut Cur<'_>,
    lineno: usize,
) -> Result<(String, Option<String>), ParseError> {
    let name = parse_er_name(cur, lineno)?;
    if !cur.eat("[") {
        return Ok((name, None));
    }
    let raw = cur
        .take_until("]")
        .ok_or_else(|| err(lineno, format!("unclosed '[' after entity '{}'", name)))?;
    let alias = clean_label_1line(&raw);
    Ok((name, (!alias.is_empty()).then_some(alias)))
}

fn parse_er_name(cur: &mut Cur<'_>, lineno: usize) -> Result<String, ParseError> {
    cur.skip_ws();
    if cur.eat("\"") {
        let name = cur
            .take_until("\"")
            .ok_or_else(|| err(lineno, "unclosed quoted entity name".to_string()))?;
        let name = decode_entities(name.trim());
        if name.is_empty() {
            return Err(err(lineno, "empty quoted entity name".to_string()));
        }
        return Ok(name);
    }
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

/// Mermaid's cardinality tokens, symbolic and spelled out, in its
/// lexer's order (the first match wins, so `one or more` is tried
/// before `one`). Either side takes any of them: the symbols merely
/// face their entity in the usual spelling (`||--o{`).
const ER_CARDS: &[(&str, Card)] = &[
    ("one or zero", Card::ZeroOne),
    ("one or more", Card::OneMany),
    ("one or many", Card::OneMany),
    ("1+", Card::OneMany),
    ("|o", Card::ZeroOne),
    ("zero or one", Card::ZeroOne),
    ("zero or more", Card::ZeroMany),
    ("zero or many", Card::ZeroMany),
    ("0+", Card::ZeroMany),
    ("}o", Card::ZeroMany),
    ("many(0)", Card::ZeroMany),
    ("many(1)", Card::OneMany),
    ("many", Card::ZeroMany),
    ("}|", Card::OneMany),
    ("one", Card::One),
    ("only one", Card::One),
    ("1", Card::One),
    ("||", Card::One),
    ("o|", Card::ZeroOne),
    ("o{", Card::ZeroMany),
    ("|{", Card::OneMany),
];

/// Mermaid's line tokens: `true` = identifying (solid).
const ER_LINES: &[(&str, bool)] = &[
    ("..", false),
    ("--", true),
    ("to", true),
    ("optionally to", false),
    (".-", false),
    ("-.", false),
];

/// The first entry of `table` the cursor starts with (letters compared
/// case-insensitively, as mermaid's lexer does); a token ending in a
/// letter or digit must not run on into a word (`one` is not `onex`).
fn er_token<T: Copy>(cur: &mut Cur<'_>, table: &[(&str, T)]) -> Option<T> {
    let rest = cur.rest();
    let (tok, v) = table.iter().find(|(t, _)| {
        rest.get(..t.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(t))
            && (!t.ends_with(|c: char| c.is_ascii_alphanumeric())
                || !rest[t.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })?;
    cur.pos += tok.len();
    Some(*v)
}

/// Crow's foot relationship operator: a cardinality, a line and a
/// cardinality, e.g. `||--o{`, `}o..o|` or `only one to zero or more`.
/// The token next to each entity is that entity's cardinality.
fn parse_rel_op(cur: &mut Cur<'_>) -> Option<(Card, bool, Card)> {
    let at = cur.pos;
    let parsed = (|| {
        let from = er_token(cur, ER_CARDS)?;
        cur.skip_ws();
        let identifying = er_token(cur, ER_LINES)?;
        cur.skip_ws();
        let to = er_token(cur, ER_CARDS)?;
        Some((from, identifying, to))
    })();
    if parsed.is_none() {
        cur.pos = at;
    }
    parsed
}

/// The attribute rows of one block line. Mermaid tokenises a block, so
/// a line may hold several rows (`string a string b`): words, key lists
/// (`PK, FK`) and `"comments"` that end at the next quote. A line
/// that does not read that way is read as one classic row — whose
/// comment, holding `"` itself, runs to the LAST quote — and when that
/// fails too, its error names the culprit (`unknown attribute key`).
fn parse_attrs(line: &str, lineno: usize) -> Result<Vec<Attr>, ParseError> {
    attrs_by_tokens(line, lineno).or_else(|_| parse_attr(line, lineno).map(|a| vec![a]))
}

/// A key word of an attribute row (`PK`, `fk`, …), any case.
fn er_key(word: &str) -> Option<Key> {
    match word.to_ascii_uppercase().as_str() {
        "PK" => Some(Key::Pk),
        "FK" => Some(Key::Fk),
        "UK" => Some(Key::Uk),
        _ => None,
    }
}

/// [`parse_attrs`]'s token reading: `type name [key[, key]…] ["comment"]`
/// repeated.
fn attrs_by_tokens(line: &str, lineno: usize) -> Result<Vec<Attr>, ParseError> {
    // (text, quoted)
    let mut toks: Vec<(String, bool)> = Vec::new();
    let mut cur = Cur::new(line);
    loop {
        cur.skip_ws();
        if cur.at_end() {
            break;
        }
        if cur.eat("\"") {
            let c = cur
                .take_until("\"")
                .ok_or_else(|| err(lineno, "unclosed attribute comment quote".to_string()))?;
            toks.push((c, true));
            continue;
        }
        let start = cur.pos;
        while let Some(c) = cur.peek() {
            if c.is_whitespace() || c == '"' {
                break;
            }
            cur.bump();
        }
        toks.push((line[start..cur.pos].to_string(), false));
    }
    let mut out = Vec::new();
    let mut i = 0;
    let word = |i: usize, what: String| -> Result<String, ParseError> {
        match toks.get(i) {
            Some((w, false)) => Ok(w.clone()),
            _ => Err(err(lineno, what)),
        }
    };
    while i < toks.len() {
        let ty = word(i, "expected an attribute type".to_string())?;
        let name = word(
            i + 1,
            format!("expected an attribute name after type '{}'", ty),
        )?;
        i += 2;
        let mut keys = Vec::new();
        // `PK,FK`, `PK, FK` and `PK , FK` all list two keys.
        while let Some((w, false)) = toks.get(i) {
            let parts: Vec<&str> = w.split(',').filter(|p| !p.is_empty()).collect();
            let ks: Option<Vec<Key>> = parts.iter().map(|p| er_key(p)).collect();
            match ks {
                Some(ks) if !(parts.is_empty() && keys.is_empty()) => keys.extend(ks),
                _ => break,
            }
            i += 1;
        }
        let comment = match toks.get(i) {
            Some((c, true)) => {
                i += 1;
                Some(c.clone())
            }
            _ => None,
        };
        out.push(Attr {
            ty,
            name,
            keys,
            comment,
        });
    }
    Ok(out)
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
        .ok_or_else(|| {
            err(
                lineno,
                format!("expected an attribute name after type '{}'", ty),
            )
        })?
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
                    format!(
                        "unknown attribute key: '{}' (expected PK, FK, or UK)",
                        other
                    ),
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
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Jison)?;
    let source: &str = &source;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;
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
            format!(
                "class block '{}' is never closed with '}}'",
                d.classes[ci].name
            ),
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
        Some(c) => (
            line[..c].trim(),
            Some(clean_label_1line(line[c + 1..].trim())),
        ),
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

/// Apply `config.sequence.*` from YAML frontmatter (see
/// [`frontmatter_config`]): mermaid's `mirrorActors`, plus flowmaid's
/// `scopedFrames`. A value that is not a YAML boolean keeps the
/// default; unknown keys (mermaid has many) are skipped.
fn apply_sequence_config(d: &mut SequenceDiagram, source: &str) {
    for (k, v) in frontmatter_config(source, "sequence") {
        let v = unquote_yaml(v);
        match k {
            "mirrorActors" => d.mirror_actors = yaml_bool(&v, d.mirror_actors),
            "scopedFrames" => d.scoped_frames = yaml_bool(&v, d.scoped_frames),
            _ => {}
        }
    }
}

/// Parse a `sequenceDiagram` body. `header_line` is the 1-indexed
/// line of the header; everything up to and including it is skipped.
fn parse_sequence(source: &str, header_line: usize) -> Result<SequenceDiagram, ParseError> {
    let mut d = SequenceDiagram::default();
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Jison)?;
    let source: &str = &source;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;
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
                    format!(
                        "autonumber arguments are not supported yet: '{}'",
                        rest.trim()
                    ),
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
                return Err(err(
                    lineno,
                    format!("unexpected text after 'end': '{}'", rest.trim()),
                ));
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
            "box",
            "rect",
            "critical",
            "option",
            "break",
            "create",
            "destroy",
            "title",
            "links",
            "link",
            "properties",
            "details",
        ];
        if let Some(kw) = UNSUPPORTED
            .iter()
            .find(|k| strip_keyword(line, k).is_some())
        {
            return Err(err(
                lineno,
                format!("sequence element '{}' is not supported yet", kw),
            ));
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
        // Error only when there's literally nothing after `as`; an
        // alias that merely CLEANS to empty (quoted whitespace,
        // `#32;`) falls back to no-alias instead of failing files
        // that used to load.
        if after.trim().is_empty() {
            return Err(err(lineno, "expected a label after 'as'".to_string()));
        }
        let l = clean_label_1line(after.trim());
        if l.is_empty() {
            None
        } else {
            Some(l)
        }
    } else {
        return Err(err(
            lineno,
            format!(
                "expected 'as <label>' after the id, found: '{}'",
                cur.rest()
            ),
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
    // Emptiness is judged on the RAW text: a note that merely CLEANS
    // to empty (quoted whitespace, `#32;`) renders as a blank chip,
    // it doesn't fail the file.
    let raw = rest[colon + 1..].trim();
    if raw.is_empty() {
        return Err(err(lineno, "note text is empty".to_string()));
    }
    let (place, text) = (rest[..colon].trim(), clean_label_1line(raw));
    let text = if text.is_empty() {
        " ".to_string()
    } else {
        text
    };
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
                return Err(err(
                    lineno,
                    "empty participant after ',' in 'note over'".to_string(),
                ))
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
                format!(
                    "expected 'over' / 'left of' / 'right of', found: '{}'",
                    place
                ),
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
            format!(
                "expected ': text' after the message target, found: '{}'",
                cur.rest()
            ),
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
                format!(
                    "'-' deactivates the sender, but '{}' is not activated",
                    from_id
                ),
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
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Langium)?;
    let source: &str = &source;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;

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
            format!(
                "unexpected text after 'pie': '{}' (expected showData and/or title)",
                rest
            ),
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
/// A `::icon(..)` line and a `:::a b` class line decorate the node
/// above them, as in mermaid. `classDef name fill:..,color:..` lines
/// (a flowmaid extension — mermaid's mindmap leaves classes to page
/// CSS) colour the nodes carrying that class.
fn parse_mindmap(source: &str, header_line: usize) -> Result<Mindmap, ParseError> {
    let mut d = Mindmap::default();
    // Stack of open ancestors as (indent, node index), shallow -> deep.
    let mut stack: Vec<(usize, usize)> = Vec::new();
    // classDefs may follow their use, as in flowcharts: resolved at the end.
    let mut class_defs: HashMap<String, NodeStyle> = HashMap::new();

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        // Decoration lines attach to the previous node, whatever their
        // indentation (mermaid's `decorateNode`); before any node there
        // is nothing to decorate.
        if let Some(rest) = trimmed.strip_prefix("::icon(") {
            // The icon runs to the first `)`, like mermaid's lexer.
            let icon = rest.split(')').next().unwrap_or("").trim();
            if let (Some(last), false) = (d.nodes.last_mut(), icon.is_empty()) {
                last.icon = Some(icon.to_string());
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(":::") {
            if let Some(last) = d.nodes.last_mut() {
                last.classes
                    .extend(rest.split_whitespace().map(str::to_string));
            }
            continue;
        }
        // `classDef name props` — only when the props parse, so a node
        // whose text merely starts with the word stays a node.
        if let Some(rest) = strip_keyword(trimmed, "classDef") {
            if let Some((names, props)) = rest.trim().split_once(char::is_whitespace) {
                if let Ok(st) = parse_props(props.trim(), lineno) {
                    for name in names.split(',').filter(|n| !n.is_empty()) {
                        class_defs.insert(name.to_string(), st.clone());
                    }
                    continue;
                }
            }
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
            icon: None,
            classes: Vec::new(),
            style: NodeStyle::default(),
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
    for n in &mut d.nodes {
        for c in &n.classes {
            if let Some(st) = class_defs.get(c) {
                n.style.apply_over(st);
            }
        }
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
    (mind_br(t), MindShape::Default)
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
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Jison)?;
    let source: &str = &source;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;
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
        // Mermaid reads the score with `Number()`, so `3.5` and `-1`
        // are scores too; clamp to the 1..=5 scale the faces draw (a
        // NaN has no place on it and stays an error).
        let exact_score: f64 = score_str
            .parse::<f64>()
            .ok()
            .filter(|v| !v.is_nan())
            .ok_or_else(|| err(lineno, format!("invalid journey score: '{}'", score_str)))?
            .clamp(1.0, 5.0);
        // `as` is exact here: the value is a whole number in 1..=5.
        let score = exact_score.round() as u8;
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
            exact_score,
        });
    }
    Ok(d)
}

// ---------------------------------------------------------------
// Timeline (`timeline`)
// ---------------------------------------------------------------

/// Direction token on the `timeline` header. Mirrors mermaid's optional
/// `LR`/`TD` suffix; anything else (including bare `timeline`) is `LR`.
fn parse_timeline_direction(line: &str) -> TimelineDirection {
    let line = line.trim();
    let rest = strip_keyword(line, "timeline").unwrap_or(line).trim();
    match rest.to_uppercase().as_str() {
        "TD" => TimelineDirection::TopDown,
        _ => TimelineDirection::LeftRight,
    }
}

/// Normalise timeline period/event text: fold `<br>` variants into line
/// breaks. Entities are left as-is — mermaid's timeline stores text raw
/// (no entity decoding), so `#35;` stays `#35;`.
fn timeline_text(s: &str) -> String {
    normalize_breaks(s).trim().to_string()
}

/// Split the event portion of a timeline row (which begins with `:`) into
/// individual event texts, mirroring mermaid's lexer: a `:` that is
/// followed by whitespace starts the next event; any other `:` (`::`,
/// `http://`) is literal text. Event text is trimmed of edge whitespace.
fn split_timeline_events(rest: &str) -> Vec<String> {
    let mut events = Vec::new();
    let chars: Vec<char> = rest.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        if chars[i] != ':' {
            break; // malformed — not an event boundary
        }
        i += 1; // consume ':'
        while i < n && chars[i].is_whitespace() {
            i += 1;
        }
        let start = i;
        // Consume text until a ':' that is followed by whitespace (or end).
        while i < n {
            if chars[i] == ':' {
                let sep = i + 1 >= n || chars[i + 1].is_whitespace();
                if sep {
                    break;
                }
            }
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        let text = timeline_text(&text);
        if !text.is_empty() {
            events.push(text);
        }
    }
    events
}

/// Parse a timeline (`timeline` / `timeline LR` / `timeline TD`).
///
/// Mirrors mermaid's timeline grammar: an optional `title`, optional
/// `section` groups, and rows of `period : event[: event...]`. A row that
/// starts with `:` (no period) continues the previous period's events; a
/// `:` in event text is literal unless followed by whitespace; `#` and
/// `%%` are comments; `<br>` variants become line breaks later at render.
fn parse_timeline(source: &str, header_line: usize) -> Result<Timeline, ParseError> {
    let mut d = Timeline::default();
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Jison)?;
    let source: &str = &source;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;
    if let Some(header) = source.lines().nth(header_line.saturating_sub(1)) {
        d.direction = parse_timeline_direction(header);
    }
    // Ensure there is always a current section to push periods into.
    let ensure_section = |d: &mut Timeline| {
        if d.sections.is_empty() {
            d.sections.push(TimelineSection {
                name: String::new(),
                periods: Vec::new(),
            });
        }
    };

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") || line.starts_with('#') {
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
            d.sections.push(TimelineSection {
                name: name.to_string(),
                periods: Vec::new(),
            });
            continue;
        }

        ensure_section(&mut d);
        let periods = &mut d.sections.last_mut().unwrap().periods;

        // Continuation: a leading `:` with no period of its own.
        if line.starts_with(':') {
            if periods.is_empty() {
                return Err(err(
                    lineno,
                    "timeline event has no preceding period".to_string(),
                ));
            }
            let events = split_timeline_events(line);
            periods.last_mut().unwrap().events.extend(events);
            continue;
        }

        // A data row: the period runs to the first `:` or `#`; a `#`
        // starts a comment (so it ends the period with no events).
        let cut = line.find(['#', ':']);
        let period = match cut {
            Some(idx) => timeline_text(&line[..idx]),
            None => timeline_text(line),
        };
        if period.is_empty() {
            return Err(err(lineno, "timeline row needs a period".to_string()));
        }
        let events = match cut {
            Some(idx) if line.as_bytes()[idx] == b':' => split_timeline_events(&line[idx..]),
            _ => Vec::new(),
        };
        periods.push(TimelinePeriod { period, events });
    }
    Ok(d)
}

// ---------------------------------------------------------------
// Git graph (`gitGraph`)
// ---------------------------------------------------------------

/// Parse a git graph. The default branch `main` exists from the start;
/// `branch` creates a new branch from the current branch's HEAD, and
/// `checkout`/`switch` moves the active branch. `merge` produces a
/// merge commit whose second parent is the named branch's HEAD.
///
/// `branch name order: n` places the branch's lane the way mermaid
/// does: lanes sort by `order`, a branch without one sitting after
/// main (and any `order: 0` branch) and before `order: 1`, branches
/// that tie keeping creation order. The resulting lane rank is what
/// each [`GitBranch::order`] holds once parsing is done.
fn parse_gitgraph(source: &str, header_line: usize) -> Result<GitGraph, ParseError> {
    let mut d = GitGraph::default();
    // The `order:` each branch was declared with, by branch index;
    // main is mermaid's `mainBranchOrder` (0).
    let mut declared_order: Vec<Option<usize>> = vec![Some(0)];
    // Accessibility statements parse (so they never misread as
    // content) but are not stored yet: this model's fields are all
    // public, so a new field would break struct-literal users.
    let (source, _acc) = extract_acc(source, header_line, false, AccFlavor::Langium)?;
    let source: &str = &source;
    if let Some(header) = source.lines().nth(header_line.saturating_sub(1)) {
        d.orientation = parse_git_orientation(header, header_line)?;
    }
    let mut used_ids = HashSet::new();

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some(rest) = strip_keyword(line, "commit") {
            let attrs = parse_git_attrs(rest, lineno, false)?;
            let id = match attrs.get("id").cloned() {
                Some(id) => id,
                None => {
                    // Default ids are 1-based, matching the common gitGraph look.
                    // Skip any numeric id already claimed by an explicit id:.
                    let mut n = d.commits.len() + 1;
                    while used_ids.contains(&n.to_string()) {
                        n += 1;
                    }
                    n.to_string()
                }
            };
            if !used_ids.insert(id.clone()) {
                return Err(err(lineno, format!("duplicate commit id: '{}'", id)));
            }
            let kind = attrs
                .get("type")
                .map(|t| parse_commit_kind(t, lineno))
                .transpose()?
                .unwrap_or_default();
            let tag = attrs.get("tag").cloned();
            let branch_idx = d.current_branch;
            let parent = d.branches[branch_idx].head;
            let seq = d.commits.len();
            let idx = d.commits.len();
            d.commits.push(GitCommit {
                id: id.clone(),
                branch: branch_idx,
                parent,
                second_parent: None,
                tag,
                kind,
                seq,
            });
            d.branches[branch_idx].head = Some(idx);
            continue;
        }

        if let Some(rest) = strip_keyword(line, "branch") {
            let (name, order) = parse_git_branch(rest, lineno)?;
            if name.is_empty() {
                return Err(err(lineno, "branch needs a name".to_string()));
            }
            if d.branches.iter().any(|b| b.name == name) {
                return Err(err(lineno, format!("branch '{}' already exists", name)));
            }
            let from_branch = d.current_branch;
            let from_commit = d.branches[from_branch].head;
            let idx = d.branches.len();
            d.branches.push(GitBranch {
                name: name.to_string(),
                parent_branch: Some(from_branch),
                parent_commit: from_commit,
                head: from_commit,
                // Creation order until the lanes are ranked below.
                order: idx,
            });
            declared_order.push(order);
            d.current_branch = idx;
            continue;
        }

        if let Some(rest) = strip_keyword(line, "checkout") {
            let name = unquote_git_name(rest);
            let idx = d
                .branches
                .iter()
                .position(|b| b.name == name)
                .ok_or_else(|| err(lineno, format!("branch '{}' does not exist", name)))?;
            d.current_branch = idx;
            continue;
        }

        if let Some(rest) = strip_keyword(line, "switch") {
            let name = unquote_git_name(rest);
            let idx = d
                .branches
                .iter()
                .position(|b| b.name == name)
                .ok_or_else(|| err(lineno, format!("branch '{}' does not exist", name)))?;
            d.current_branch = idx;
            continue;
        }

        if let Some(rest) = strip_keyword(line, "merge") {
            // `merge branch [id: "x"] [tag: "y"] [type: HIGHLIGHT]`.
            let attrs = parse_git_attrs(rest, lineno, true)?;
            let branch_name = attrs
                .get("__head")
                .cloned()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| err(lineno, "merge needs a branch name".to_string()))?;
            let src_idx = d
                .branches
                .iter()
                .position(|b| b.name == branch_name)
                .ok_or_else(|| err(lineno, format!("branch '{}' does not exist", branch_name)))?;
            if src_idx == d.current_branch {
                return Err(err(lineno, "cannot merge a branch into itself".to_string()));
            }
            let src_head = d.branches[src_idx].head.ok_or_else(|| {
                err(
                    lineno,
                    format!("branch '{}' has no commits to merge", branch_name),
                )
            })?;
            let branch_idx = d.current_branch;
            let parent = d.branches[branch_idx].head;
            let id = match attrs.get("id").cloned() {
                Some(id) => id,
                None => {
                    let mut n = d.commits.len() + 1;
                    while used_ids.contains(&n.to_string()) {
                        n += 1;
                    }
                    n.to_string()
                }
            };
            if !used_ids.insert(id.clone()) {
                return Err(err(lineno, format!("duplicate commit id: '{}'", id)));
            }
            let tag = attrs.get("tag").cloned();
            // Mermaid lets a merge carry a commit type too; it decides
            // the merge commit's glyph like any commit's.
            let kind = attrs
                .get("type")
                .map(|t| parse_commit_kind(t, lineno))
                .transpose()?
                .unwrap_or_default();
            let seq = d.commits.len();
            let idx = d.commits.len();
            d.commits.push(GitCommit {
                id,
                branch: branch_idx,
                parent,
                second_parent: Some(src_head),
                tag,
                kind,
                seq,
            });
            d.branches[branch_idx].head = Some(idx);
            continue;
        }

        return Err(err(lineno, format!("unknown gitGraph command: '{}'", line)));
    }

    // Rank the lanes: by declared order, an undeclared branch between
    // order 0 and order 1, ties in creation order. (Mermaid keys an
    // undeclared branch `0.<index>`, which puts the 10th branch level
    // with the 1st; creation order is what that key means to say.)
    let mut by_lane: Vec<usize> = (0..d.branches.len()).collect();
    by_lane.sort_by_key(|&i| match declared_order[i] {
        Some(n) => (n, 0, i),
        None => (0, 1, i),
    });
    for (lane, i) in by_lane.into_iter().enumerate() {
        d.branches[i].order = lane;
    }

    Ok(d)
}

/// `name [order: n]` after `branch`: the name (quotes stripped) and the
/// declared lane order. A name may hold spaces (flowmaid always took
/// the whole rest of the line); only a trailing `order: <n>` is split
/// off.
fn parse_git_branch(rest: &str, lineno: usize) -> Result<(&str, Option<usize>), ParseError> {
    let rest = rest.trim();
    let split = rest
        .match_indices("order:")
        .map(|(i, _)| i)
        .filter(|&i| i > 0 && rest[..i].ends_with(char::is_whitespace))
        .last();
    let Some(at) = split else {
        return Ok((unquote_git_name(rest), None));
    };
    let value = rest[at + "order:".len()..].trim();
    let order = value.parse::<usize>().map_err(|_| {
        err(
            lineno,
            format!(
                "branch order must be a non-negative integer, got '{}'",
                value
            ),
        )
    })?;
    Ok((unquote_git_name(&rest[..at]), Some(order)))
}

/// A branch reference as mermaid writes it: bare, or in double quotes.
fn unquote_git_name(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s)
}

/// Parse `key: "value"` attributes for git commands.
///
/// When `allow_head` is true, the leading bare token (if any) is
/// returned under the synthetic key `__head` — this is used by `merge`
/// to read the source branch name. `commit` passes `false` so that any
/// bare token is parsed as an attribute key and unknown keys surface as
/// errors.
fn parse_git_attrs(
    text: &str,
    lineno: usize,
    allow_head: bool,
) -> Result<HashMap<String, String>, ParseError> {
    let mut out = HashMap::new();
    let text = text.trim();
    if text.is_empty() {
        return Ok(out);
    }

    // The first whitespace-separated token is the head argument
    // (branch name for merge/branch/checkout) unless it is itself an
    // attribute key like `id:`.
    let rest: &str = if allow_head {
        // A quoted head (`merge "my branch" id: "m"`) may hold spaces.
        let quoted = text
            .strip_prefix('"')
            .and_then(|t| t.find('"').map(|end| (&t[..end], &t[end + 1..])));
        if let Some((head, tail)) = quoted {
            out.insert("__head".to_string(), head.to_string());
            tail.trim_start()
        } else if let Some((first, tail)) = text.split_once(char::is_whitespace) {
            if is_git_attr_key(first.trim_end_matches(':')) {
                text
            } else {
                out.insert("__head".to_string(), first.to_string());
                tail.trim_start()
            }
        } else if is_git_attr_key(text.trim_end_matches(':')) {
            text
        } else {
            out.insert("__head".to_string(), text.to_string());
            ""
        }
    } else {
        text
    };

    // Parse `key: "value"` / `key: value` / `key:"value"` pairs from `rest`.
    let mut cur = Cur::new(rest);
    while !cur.at_end() {
        cur.skip_ws();
        if cur.at_end() {
            break;
        }
        // Mermaid's `commit "text"`: a bare string is the commit
        // message, the same as `msg: "text"`.
        if !allow_head && cur.peek() == Some('"') {
            let value = parse_git_value(&mut cur, lineno)?;
            out.insert("msg".to_string(), value);
            continue;
        }
        let key = cur.take_until(":").ok_or_else(|| {
            err(
                lineno,
                format!("expected attribute key:value, got '{}'", rest),
            )
        })?;
        let key = key.trim();
        if key.is_empty() {
            return Err(err(lineno, "empty attribute key".to_string()));
        }
        if !is_git_attr_key(key) {
            return Err(err(
                lineno,
                format!("unknown gitGraph attribute: '{}'", key),
            ));
        }
        cur.skip_ws();
        let value = parse_git_value(&mut cur, lineno)?;
        out.insert(key.to_string(), value);
        cur.skip_ws();
    }
    Ok(out)
}

/// Attribute keys `commit` / `merge` take. `msg` (a commit message) is
/// read and dropped: mermaid draws no message either.
fn is_git_attr_key(s: &str) -> bool {
    matches!(s, "id" | "tag" | "type" | "msg")
}

fn parse_git_value(cur: &mut Cur<'_>, lineno: usize) -> Result<String, ParseError> {
    cur.skip_ws();
    if cur.eat("\"") {
        let value = cur
            .take_until("\"")
            .ok_or_else(|| err(lineno, "attribute value quote is never closed".to_string()))?;
        return Ok(value);
    }
    // Bare token value (e.g. `type: HIGHLIGHT`).
    let start = cur.pos;
    while !cur.at_end() && !cur.rest().starts_with(char::is_whitespace) {
        cur.bump();
    }
    let value = &cur.s[start..cur.pos];
    if value.is_empty() {
        return Err(err(lineno, "missing attribute value".to_string()));
    }
    Ok(value.to_string())
}

fn parse_commit_kind(s: &str, lineno: usize) -> Result<CommitKind, ParseError> {
    match s.trim().to_uppercase().as_str() {
        "NORMAL" => Ok(CommitKind::Normal),
        "REVERSE" => Ok(CommitKind::Reverse),
        "HIGHLIGHT" => Ok(CommitKind::Highlight),
        other => Err(err(
            lineno,
            format!(
                "unknown commit type: '{}' (expected NORMAL, REVERSE, HIGHLIGHT)",
                other
            ),
        )),
    }
}

// ---------------------------------------------------------------
// Architecture diagram (`architecture-beta`)
// ---------------------------------------------------------------

/// Parse an architecture-beta diagram. Groups, services and junctions
/// are declared with `group` / `service` / `junction`, and edges
/// connect named port sides (`svc:L -- R:other`, labelled
/// `svc:L -[label]- R:other`).
fn parse_architecture(source: &str, header_line: usize) -> Result<Architecture, ParseError> {
    let mut d = Architecture::default();
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Langium)?;
    d.acc_title = acc.title;
    d.acc_descr = acc.descr;
    let source: &str = &source;
    let mut group_ids: HashMap<String, usize> = HashMap::new();
    let mut service_ids: HashMap<String, usize> = HashMap::new();

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some(rest) = strip_keyword(line, "group") {
            let (id, icon, title, parent) = parse_arch_decl(rest, lineno, &group_ids, "group")?;
            if group_ids.contains_key(&id) {
                return Err(err(lineno, format!("duplicate group id: '{}'", id)));
            }
            let idx = d.groups.len();
            d.groups.push(ArchGroup {
                id: id.clone(),
                title,
                icon,
                parent,
            });
            group_ids.insert(id, idx);
            continue;
        }

        if let Some(rest) = strip_keyword(line, "service") {
            let (id, icon, title, parent) = parse_arch_decl(rest, lineno, &group_ids, "service")?;
            if service_ids.contains_key(&id) {
                return Err(err(lineno, format!("duplicate service id: '{}'", id)));
            }
            let idx = d.services.len();
            d.services.push(ArchService {
                id: id.clone(),
                title,
                icon,
                group: parent,
                junction: false,
            });
            service_ids.insert(id, idx);
            continue;
        }

        if let Some(rest) = strip_keyword(line, "junction") {
            let (id, parent) = parse_arch_junction(rest, lineno, &group_ids)?;
            if service_ids.contains_key(&id) {
                return Err(err(lineno, format!("duplicate junction id: '{}'", id)));
            }
            let idx = d.services.len();
            d.services.push(ArchService {
                id: id.clone(),
                title: String::new(),
                icon: None,
                group: parent,
                junction: true,
            });
            service_ids.insert(id, idx);
            continue;
        }

        // mermaid's `align row a b …` / `align column a b …` layout
        // hints: accepted so such diagrams render, not applied — the
        // port-driven layout already lines up what the edges connect.
        if let Some(rest) = strip_keyword(line, "align") {
            let mut words = rest.split_whitespace();
            let dir_ok = matches!(words.next(), Some("row") | Some("column"));
            let ids: Vec<&str> = words.collect();
            if !dir_ok || ids.len() < 2 {
                return Err(err(
                    lineno,
                    "expected `align row|column <id> <id> …`".to_string(),
                ));
            }
            if let Some(missing) = ids.iter().find(|id| !service_ids.contains_key(**id)) {
                return Err(err(lineno, format!("service '{}' not found", missing)));
            }
            continue;
        }

        if line.contains("--") || line.contains("-[") {
            let e = parse_arch_edge(line, lineno, &service_ids, &group_ids, &d.services)?;
            d.edges.push(e);
            continue;
        }

        return Err(err(
            lineno,
            format!("unknown architecture statement: '{}'", line),
        ));
    }

    Ok(d)
}

/// Parse `id(icon)[title] in parent`. `icon`, `[title]` and
/// `in parent` are optional (as in mermaid); `parent` must name a group
/// already declared. A title may be quoted (`["Edge, west"]`).
fn parse_arch_decl(
    rest: &str,
    lineno: usize,
    groups: &HashMap<String, usize>,
    kind: &str,
) -> Result<(String, Option<String>, String, Option<usize>), ParseError> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(err(lineno, format!("{} needs an id", kind)));
    }
    let id_end = rest
        .find(|c: char| c == '(' || c == '[' || c.is_whitespace())
        .unwrap_or(rest.len());
    let id = rest[..id_end].trim().to_string();
    if id.is_empty() {
        return Err(err(lineno, format!("{} needs an id", kind)));
    }
    let mut tail = &rest[id_end..];
    let icon = if tail.starts_with('(') {
        let close = tail
            .find(')')
            .ok_or_else(|| err(lineno, "unclosed icon '()'".to_string()))?;
        let ic = tail[1..close].trim().to_string();
        tail = &tail[close + 1..];
        Some(ic)
    } else {
        None
    };
    let title = if tail.starts_with('[') {
        let close = tail
            .find(']')
            .ok_or_else(|| err(lineno, "unclosed title '['".to_string()))?;
        let t = arch_unquote(&tail[1..close]);
        tail = &tail[close + 1..];
        t
    } else {
        String::new()
    };
    let tail = tail.trim();
    let parent = arch_parent(tail, lineno, groups)?;
    Ok((id, icon, title, parent))
}

/// Parse `id` / `id in parent` after the `junction` keyword.
fn parse_arch_junction(
    rest: &str,
    lineno: usize,
    groups: &HashMap<String, usize>,
) -> Result<(String, Option<usize>), ParseError> {
    let rest = rest.trim();
    let id_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let id = &rest[..id_end];
    if id.is_empty() || id.contains(['(', '[', ':', '{']) {
        return Err(err(lineno, "junction needs a plain id".to_string()));
    }
    let parent = arch_parent(rest[id_end..].trim(), lineno, groups)?;
    Ok((id.to_string(), parent))
}

/// The optional `in parent` that closes a declaration; anything else
/// left over is an error.
fn arch_parent(
    tail: &str,
    lineno: usize,
    groups: &HashMap<String, usize>,
) -> Result<Option<usize>, ParseError> {
    if let Some(pname) = strip_keyword(tail, "in") {
        let pname = pname.trim();
        if pname.is_empty() {
            return Err(err(lineno, "missing parent group name".to_string()));
        }
        Ok(Some(*groups.get(pname).ok_or_else(|| {
            err(lineno, format!("parent group '{}' not found", pname))
        })?))
    } else if !tail.is_empty() {
        Err(err(lineno, format!("unexpected trailing text: '{}'", tail)))
    } else {
        Ok(None)
    }
}

/// A `[…]` title body: trimmed, with one pair of matching surrounding
/// quotes removed (mermaid accepts `["…"]` and `['…']`).
fn arch_unquote(s: &str) -> String {
    let t = s.trim();
    let quoted = t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')));
    if quoted {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Parse an edge such as `svc1:L -- R:svc2`, `a:T --> B:b`,
/// `svc1:L <--> R:svc2` or the labelled `a:R -[calls]-> L:b`.
/// Mermaid's `{group}` modifier after a service id (`svc1{group}:L`)
/// attaches that end to the service's group frame; flowmaid's named
/// form `svc1{g}:L` does the same and checks that `svc1` sits in `g`.
fn parse_arch_edge(
    line: &str,
    lineno: usize,
    services: &HashMap<String, usize>,
    groups: &HashMap<String, usize>,
    arch_services: &[ArchService],
) -> Result<ArchEdge, ParseError> {
    // The connector: a labelled `-[…]-`, else a plain `--`.
    let (mut left, mut right, title) = match line.find("-[") {
        Some(open) => {
            let close = line[open..]
                .find("]-")
                .map(|c| open + c)
                .ok_or_else(|| err(lineno, "unclosed edge label '-['".to_string()))?;
            let title = arch_unquote(&line[open + 2..close]);
            (
                line[..open].trim(),
                line[close + 2..].trim(),
                Some(title).filter(|t| !t.is_empty()),
            )
        }
        None => {
            let idx = line
                .find("--")
                .ok_or_else(|| err(lineno, "edge must contain '--'".to_string()))?;
            (line[..idx].trim(), line[idx + 2..].trim(), None)
        }
    };

    let arrow_start = left.ends_with('<');
    if arrow_start {
        left = left[..left.len() - 1].trim_end();
    }
    let arrow_end = right.starts_with('>');
    if arrow_end {
        right = right[1..].trim_start();
    }

    let (from_id, from_group, from_side) = parse_arch_endpoint_left(left, lineno)?;
    let (to_side, to_group, to_id) = parse_arch_endpoint_right(right, lineno)?;

    let from = *services
        .get(&from_id)
        .ok_or_else(|| err(lineno, format!("service '{}' not found", from_id)))?;
    let to = *services
        .get(&to_id)
        .ok_or_else(|| err(lineno, format!("service '{}' not found", to_id)))?;

    let from_group =
        validate_arch_group_qualifier(from, from_group, "from", lineno, groups, arch_services)?;
    let to_group =
        validate_arch_group_qualifier(to, to_group, "to", lineno, groups, arch_services)?;

    Ok(ArchEdge {
        from,
        from_side,
        to,
        to_side,
        arrow_start,
        arrow_end,
        from_group,
        to_group,
        title,
    })
}

/// Check an endpoint's `{…}` qualifier and say whether the end attaches
/// to the service's group frame. Mermaid's literal `{group}` names the
/// service's own group (a service outside any group keeps its own
/// box); any other name must be the group the service sits in.
fn validate_arch_group_qualifier(
    service_idx: usize,
    qualifier: Option<String>,
    side: &str,
    lineno: usize,
    groups: &HashMap<String, usize>,
    arch_services: &[ArchService],
) -> Result<bool, ParseError> {
    let Some(name) = qualifier else {
        return Ok(false);
    };
    if name == "group" && !groups.contains_key("group") {
        return Ok(arch_services[service_idx].group.is_some());
    }
    let group_idx = *groups.get(&name).ok_or_else(|| {
        err(
            lineno,
            format!("{} group qualifier '{}' not found", side, name),
        )
    })?;
    if arch_services[service_idx].group != Some(group_idx) {
        return Err(err(
            lineno,
            format!(
                "service '{}' is not inside group '{}'",
                arch_services[service_idx].id, name
            ),
        ));
    }
    Ok(true)
}

fn parse_arch_endpoint_left(
    s: &str,
    lineno: usize,
) -> Result<(String, Option<String>, ArchSide), ParseError> {
    let s = s.trim();
    let colon = s
        .rfind(':')
        .ok_or_else(|| err(lineno, format!("expected id:side, got '{}'", s)))?;
    let (id, group) = parse_arch_id_with_group(&s[..colon], lineno)?;
    let side = parse_arch_side(&s[colon + 1..], lineno)?;
    Ok((id, group, side))
}

fn parse_arch_endpoint_right(
    s: &str,
    lineno: usize,
) -> Result<(ArchSide, Option<String>, String), ParseError> {
    let s = s.trim();
    let colon = s
        .find(':')
        .ok_or_else(|| err(lineno, format!("expected side:id, got '{}'", s)))?;
    let side = parse_arch_side(&s[..colon], lineno)?;
    let (id, group) = parse_arch_id_with_group(&s[colon + 1..], lineno)?;
    Ok((side, group, id))
}

fn parse_arch_id_with_group(
    s: &str,
    lineno: usize,
) -> Result<(String, Option<String>), ParseError> {
    let s = s.trim();
    let Some(open) = s.find('{') else {
        return Ok((s.to_string(), None));
    };
    let close = s
        .rfind('}')
        .filter(|c| *c > open)
        .ok_or_else(|| err(lineno, format!("unclosed group qualifier in '{}'", s)))?;
    let id = s[..open].trim().to_string();
    let group = s[open + 1..close].trim().to_string();
    if id.is_empty() {
        return Err(err(
            lineno,
            "service id missing before group qualifier".to_string(),
        ));
    }
    if group.is_empty() {
        return Err(err(lineno, "empty group qualifier".to_string()));
    }
    Ok((id, Some(group)))
}

fn parse_arch_side(s: &str, lineno: usize) -> Result<ArchSide, ParseError> {
    match s.trim().to_ascii_uppercase().as_str() {
        "T" => Ok(ArchSide::T),
        "B" => Ok(ArchSide::B),
        "L" => Ok(ArchSide::L),
        "R" => Ok(ArchSide::R),
        other => Err(err(
            lineno,
            format!("unknown port side: '{}' (expected T, B, L, R)", other),
        )),
    }
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
    let (source, acc) = extract_acc(source, header_line, false, AccFlavor::Jison)?;
    let source: &str = &source;
    g.acc_title = acc.title;
    g.acc_descr = acc.descr;

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
    // A block note being read: where it attaches, and its lines so far.
    // `None` inside the block = a note form we keep skipping.
    let mut note_block: Option<Option<(StateNote, Vec<String>)>> = None;
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
        if let Some(open) = note_block.as_mut() {
            if line.eq_ignore_ascii_case("end note") {
                if let Some((at, lines)) = note_block.take().flatten() {
                    state_note(&mut g, &stack, &sub_ids, at, &lines.join("\n"));
                }
            } else if let Some((_, lines)) = open {
                lines.push(note_text(line));
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
        // Notes, mermaid-style: a yellow card joined to its state by a
        // dotted line. `note left of X : text` is one line; without a
        // ':' the text runs on the next lines until `end note`.
        // `note "text" as N` is a free-standing card named N. Any other
        // note form is skipped as before (a block one to `end note`).
        if let Some(rest) = strip_keyword(line, "note") {
            let rest = rest.trim();
            if let Some((text, id)) = floating_note(rest) {
                let idx = state_node(&mut g, &stack, id, Some(text), Some(Shape::Rect));
                g.nodes[idx].style = note_style();
                continue;
            }
            let (head, text) = match rest.split_once(':') {
                Some((h, t)) => (h, Some(t)),
                None => (rest, None),
            };
            let at = state_note_head(head);
            match (at, text) {
                (Some(at), Some(t)) => state_note(&mut g, &stack, &sub_ids, at, &note_text(t)),
                (at, None) => note_block = Some(at.map(|a| (a, Vec::new()))),
                (None, Some(_)) => {}
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
                    ends: EdgeEnds::default(),
                    meta: EdgeMeta::default(),
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
            format!(
                "state block '{}' is never closed with '}}'",
                g.subgraphs[si].id
            ),
        ));
    }
    // A block note left open runs to the end of the source: draw what
    // it swallowed, so the missing `end note` shows instead of the rest
    // of the diagram vanishing.
    if let Some(Some((at, lines))) = note_block {
        state_note(&mut g, &stack, &sub_ids, at, &lines.join("\n"));
    }
    Ok(g)
}

/// Where a `note left of X` / `note right of X` attaches: X (a state or
/// a composite id) and whether the card goes before it along the flow.
struct StateNote {
    target: String,
    before: bool,
}

/// The head of a positioned note (`left of X`, `right of X`, before any
/// `:`), or `None` for any other form.
fn state_note_head(head: &str) -> Option<StateNote> {
    let (before, rest) = if let Some(r) = strip_keyword(head.trim(), "left") {
        (true, r)
    } else {
        (false, strip_keyword(head.trim(), "right")?)
    };
    let target = strip_keyword(rest.trim(), "of")?.trim();
    is_class_ident(target).then(|| StateNote {
        target: target.to_string(),
        before,
    })
}

/// `"text" as N` after `note`: the card's text and its id.
fn floating_note(rest: &str) -> Option<(String, &str)> {
    let body = rest.strip_prefix('"')?;
    let close = body.find('"')?;
    let id = strip_keyword(body[close + 1..].trim(), "as")?.trim();
    is_class_ident(id).then(|| (note_text(&body[..close]), id))
}

/// Note text: cleaned like a label, `<br/>` breaks kept as lines, each
/// line trimmed (`a <br/> b` is "a" over "b").
fn note_text(s: &str) -> String {
    clean_label(s)
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A note card's colours ([`crate::style::NOTE_STYLE`]).
fn note_style() -> NodeStyle {
    let crate::style::ShapeStyle { fill, stroke } = crate::style::NOTE_STYLE;
    NodeStyle {
        fill: Some(fill.to_string()),
        stroke: Some(stroke.to_string()),
        ..NodeStyle::default()
    }
}

/// Add a note card with `text` and the dotted line joining it to its
/// state, mermaid-style: a node of its own (so it is laid out, routed,
/// picked and dragged like any box) in the state's own box, linked by a
/// `DottedOpen` edge that runs from the state to a `right of` card (one
/// layer after it) and from a `left of` card to the state (one layer
/// before it, see [`Graph::lead_nodes`]). A note on a composite links
/// to its frame and sits in the composite's parent box.
fn state_note(
    g: &mut Graph,
    stack: &[(usize, usize)],
    sub_ids: &HashMap<String, usize>,
    at: StateNote,
    text: &str,
) {
    let target = match sub_ids.get(&at.target) {
        Some(&si) => End::Sub(si),
        None => End::Node(state_node(g, stack, &at.target, None, None)),
    };
    // The box the card joins: the state's own, or the composite's
    // parent (the open block when the composite is not declared yet).
    let owner = match target {
        End::Node(v) => g.subgraphs.iter().position(|s| s.nodes.contains(&v)),
        End::Sub(si) => match g.subgraphs.get(si) {
            Some(s) => s.parent,
            None => stack.last().map(|&(s, _)| s),
        },
    };
    // A fresh id the source cannot have used yet (`__start_*` style).
    let mut k = g.nodes.len();
    while g.node_index(&format!("__note_{k}")).is_some() {
        k += 1;
    }
    let note = g.ensure_node(
        &format!("__note_{k}"),
        Some(text.to_string()),
        Some(Shape::Rect),
    );
    g.nodes[note].style = note_style();
    if let Some(si) = owner {
        g.subgraphs[si].nodes.push(note);
    }
    if at.before {
        g.lead_nodes.push(note);
    }
    let card = End::Node(note);
    let (from, to) = if at.before {
        (card, target)
    } else {
        (target, card)
    };
    match (from, to) {
        (End::Node(a), End::Node(b)) => g.add_edge(a, b, None, EdgeKind::DottedOpen),
        (from, to) => g.sub_edges.push(SubEdge {
            from,
            to,
            label: None,
            kind: EdgeKind::DottedOpen,
            ends: EdgeEnds::default(),
            meta: EdgeMeta::default(),
        }),
    }
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

// ------------------------------------------------------------------
// Sankey
// ------------------------------------------------------------------

/// The raw frontmatter body — the lines BETWEEN the `---` fences, or
/// `None` when there is no frontmatter. Fence detection is delegated to
/// the same [`fence_indent`] rule [`strip_frontmatter`] uses, so the two
/// can never disagree about where the block ends.
fn frontmatter_body(source: &str) -> Option<Vec<&str>> {
    let mut lines = source.lines().enumerate();
    let (_, first) = lines.next()?;
    let open_indent = fence_indent(first)?;
    let close = lines.find_map(|(i, l)| (fence_indent(l) == Some(open_indent)).then_some(i))?;
    Some(source.lines().take(close).skip(1).collect())
}

/// Read `config.sankey.*` out of YAML frontmatter (see
/// [`frontmatter_section`]).
fn sankey_config(source: &str) -> SankeyConfig {
    let mut cfg = SankeyConfig::default();
    // Indent of a `nodeColors:` key whose block map follows. Its entries
    // are node labels, so they must never reach `apply_sankey_key` — a
    // node called `width` would otherwise resize the canvas.
    let mut colors_at: Option<usize> = None;
    for (ind, t) in frontmatter_section(source, "sankey") {
        if let Some(at) = colors_at {
            if ind > at {
                if let Some((k, v)) = yaml_pair(t) {
                    set_node_color(&mut cfg, k, unquote_yaml(v));
                }
                continue;
            }
            colors_at = None;
        }
        if let Some((k, v)) = yaml_pair(t) {
            if k == "nodeColors" {
                let v = v.trim();
                if v.is_empty() {
                    colors_at = Some(ind);
                } else if let Some(map) = v
                    .strip_prefix('{')
                    .and_then(|m| m.trim_end().strip_suffix('}'))
                {
                    for item in yaml_flow_items(map) {
                        if let Some((k, v)) = yaml_pair(item) {
                            set_node_color(&mut cfg, k, unquote_yaml(v));
                        }
                    }
                }
                continue;
            }
            apply_sankey_key(&mut cfg, k, v);
        }
    }
    cfg
}

/// The scalar `key: value` pairs (key trimmed, value raw — run it
/// through [`unquote_yaml`]) of the `config.<section>:` block in YAML
/// frontmatter, in source order; empty when there is none. Nested
/// lines are flattened, so callers see every `key: value` under the
/// section (see [`frontmatter_section`]).
fn frontmatter_config<'a>(source: &'a str, section: &str) -> Vec<(&'a str, &'a str)> {
    frontmatter_section(source, section)
        .into_iter()
        .filter_map(|(_, t)| t.split_once(':').map(|(k, v)| (k.trim(), v)))
        .collect()
}

/// Every line inside the `config.<section>:` block of YAML frontmatter,
/// as (indent, trimmed text) in source order — blank and `#` comment
/// lines left out; empty when there is no such block. The indent lets a
/// caller follow a nested block map (sankey's `nodeColors`).
///
/// Deliberately NOT a YAML parser: it walks indentation for the
/// `config:` -> `<section>:` path and hands back the lines under it.
/// Richer YAML (anchors, block scalars) is ignored rather than guessed
/// at, and callers skip unknown keys mermaid-style so a file carrying
/// config for other diagrams still renders.
fn frontmatter_section<'a>(source: &'a str, section: &str) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    let Some(body) = frontmatter_body(source) else {
        return out;
    };
    let indent_of = |l: &str| l.len() - l.trim_start().len();
    // Indent of `config:` / `<section>:` once seen. A line at or left
    // of a recorded indent has closed that block.
    let mut config_at: Option<usize> = None;
    // Indent shared by `config:`'s DIRECT children, learnt from the
    // first one seen. Without it a `sankey:` nested under some other
    // namespace (`config.themeVariables.sankey`) would reconfigure the
    // diagram.
    let mut child_at: Option<usize> = None;
    let mut section_at: Option<usize> = None;
    for line in body {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let ind = indent_of(line);
        if let Some(inner) = section_at {
            if ind > inner {
                out.push((ind, t));
                continue;
            }
            section_at = None;
        }
        if let Some(outer) = config_at {
            if ind > outer {
                let direct = *child_at.get_or_insert(ind);
                if ind == direct && t.strip_suffix(':') == Some(section) {
                    section_at = Some(ind);
                }
                continue;
            }
            config_at = None;
            child_at = None;
        }
        if t == "config:" {
            config_at = Some(ind);
        }
    }
    out
}

/// Apply one `config.sankey.<key>: <value>` pair. A value that does not
/// parse leaves the mermaid default in place rather than failing the
/// whole diagram — config is decoration, not structure.
/// A config number, accepted only when it is finite and non-negative.
///
/// `f64::from_str` happily parses `NaN`, `inf` and `-inf`, and any of
/// those reaches the SVG as `x="NaN"` or `viewBox="0 0 600 inf"`, which
/// no renderer can read. `parse_sankey` already refuses a non-finite
/// LINK value; config numbers get the same guard.
fn sankey_number(v: &str) -> Option<f64> {
    v.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0)
}

fn apply_sankey_key(cfg: &mut SankeyConfig, key: &str, raw: &str) {
    let v = unquote_yaml(raw);
    match key {
        "width" => {
            if let Some(n) = sankey_number(&v) {
                cfg.width = n;
            }
        }
        "height" => {
            if let Some(n) = sankey_number(&v) {
                cfg.height = n;
            }
        }
        "nodeWidth" => {
            if let Some(n) = sankey_number(&v) {
                cfg.node_width = n;
            }
        }
        "nodePadding" => {
            if let Some(n) = sankey_number(&v) {
                cfg.node_padding = n;
            }
        }
        "showValues" => cfg.show_values = yaml_bool(&v, cfg.show_values),
        "prefix" => cfg.prefix = v,
        "suffix" => cfg.suffix = v,
        // Keywords are matched without regard to case, like the YAML
        // booleans above — `Gradient` used to fall through to a CSS
        // colour named "Gradient" and paint the ribbons black. A value
        // that is NOT a keyword keeps its original spelling, because it
        // is a colour and CSS `var(--Name)` is case-sensitive.
        "linkColor" => {
            cfg.link_color = match v.to_ascii_lowercase().as_str() {
                "" => return,
                "source" => SankeyLinkColor::Source,
                "target" => SankeyLinkColor::Target,
                "gradient" => SankeyLinkColor::Gradient,
                _ => SankeyLinkColor::Fixed(v),
            }
        }
        "nodeAlignment" => {
            cfg.node_alignment = match v.to_ascii_lowercase().as_str() {
                "left" => SankeyAlignment::Left,
                "right" => SankeyAlignment::Right,
                "center" => SankeyAlignment::Center,
                _ => SankeyAlignment::Justify,
            }
        }
        "labelStyle" => {
            cfg.label_style = match v.to_ascii_lowercase().as_str() {
                "outlined" => SankeyLabelStyle::Outlined,
                _ => SankeyLabelStyle::Legacy,
            }
        }
        // Unknown keys are skipped, so extra mermaid config never breaks
        // a render.
        _ => {}
    }
}

/// Split a YAML `key: value` line at the colon that ends the key. A
/// quoted key (`"Industry, light": "#abc"`) ends at its closing quote,
/// so a `:` or `,` inside it stays part of the name; the key comes back
/// unquoted and trimmed, the value raw (callers unquote it).
fn yaml_pair(t: &str) -> Option<(&str, &str)> {
    let t = t.trim();
    if let Some(q) = t.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let end = t[1..].find(q)? + 1;
        let rest = t[end + 1..].trim_start().strip_prefix(':')?;
        return Some((&t[1..end], rest));
    }
    let (k, v) = t.split_once(':')?;
    Some((k.trim(), v))
}

/// The comma-separated items of a YAML flow map body (`a: 1, "b, c": 2`
/// without the braces), splitting only on commas outside quotes.
fn yaml_flow_items(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (i, c) in body.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == ',' => {
                out.push(body[start..i].trim());
                start = i + 1;
            }
            None => {}
        }
    }
    out.push(body[start..].trim());
    out.retain(|s| !s.is_empty());
    out
}

/// Record one `nodeColors` entry. An empty or `null` colour is skipped
/// (the node keeps the palette); a repeated label keeps its last colour,
/// as a YAML loader that tolerates duplicate keys would.
fn set_node_color(cfg: &mut SankeyConfig, label: &str, color: String) {
    let color = color.trim().to_string();
    if label.is_empty() || color.is_empty() || color == "null" || color == "~" {
        return;
    }
    if let Some(slot) = cfg.node_colors.iter_mut().find(|(l, _)| l == label) {
        slot.1 = color;
    } else {
        cfg.node_colors.push((label.to_string(), color));
    }
}

/// YAML 1.1 booleans. `showValues: False` and `showValues: no` mean the
/// same thing as `false`, so matching only the lowercase spelling would
/// silently ignore the setting.
fn yaml_bool(v: &str, default: bool) -> bool {
    match v.trim().to_ascii_lowercase().as_str() {
        "false" | "no" | "off" | "n" | "0" => false,
        "true" | "yes" | "on" | "y" | "1" => true,
        _ => default,
    }
}

/// Strip one layer of YAML quoting. A quoted scalar ends at its closing
/// quote, so a trailing comment is dropped WITHOUT eating a `#` that is
/// part of the value; an unquoted scalar ends at a ` #` comment.
fn unquote_yaml(raw: &str) -> String {
    let raw = raw.trim();
    if let Some(q) = raw.chars().next().filter(|c| *c == '"' || *c == '\'') {
        if let Some(end) = raw
            .char_indices()
            .skip(1)
            .find(|(_, c)| *c == q)
            .map(|(i, _)| i)
        {
            return raw[1..end].to_string();
        }
        // Unterminated — fall through and treat it as a plain scalar.
    }
    match raw.split_once(" #") {
        Some((head, _)) => head.trim().to_string(),
        None => raw.to_string(),
    }
}

/// Split one CSV row into fields, mermaid-style: a field may be double
/// quoted, and inside quotes `""` is one literal `"`. Unquoted fields
/// are trimmed; quoted ones keep their spacing.
fn split_csv_row(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        while matches!(chars.peek(), Some(' ') | Some('\t')) {
            chars.next();
        }
        let mut field = String::new();
        if chars.peek() == Some(&'"') {
            chars.next();
            loop {
                match chars.next() {
                    Some('"') if chars.peek() == Some(&'"') => {
                        chars.next();
                        field.push('"');
                    }
                    Some('"') => break,
                    Some(c) => field.push(c),
                    None => return Err("unclosed '\"' in sankey row".to_string()),
                }
            }
            // Only padding may sit between the closing quote and the
            // separator. Silently eating anything else would let a typo
            // change which node a flow attaches to.
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                if !c.is_whitespace() {
                    return Err(format!(
                        "unexpected '{c}' after a closing '\"' in a sankey row"
                    ));
                }
                chars.next();
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                field.push(c);
                chars.next();
            }
            field = field.trim_end().to_string();
        }
        out.push(field);
        if chars.next().is_none() {
            break;
        }
    }
    Ok(out)
}

/// Parse a sankey diagram: a CSV body of `source,target,value` rows.
/// Nodes are created on first mention and keyed by their label, exactly
/// as mermaid does, so repeating a label reuses the node.
fn parse_sankey(
    source: &str,
    header_line: usize,
    config: SankeyConfig,
) -> Result<SankeyDiagram, ParseError> {
    let mut d = SankeyDiagram {
        config,
        ..Default::default()
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
        let fields = split_csv_row(line).map_err(|m| err(lineno, m))?;
        if fields.len() != 3 {
            return Err(err(
                lineno,
                format!(
                    "a sankey row is 'source,target,value' — found {} field{}",
                    fields.len(),
                    if fields.len() == 1 { "" } else { "s" }
                ),
            ));
        }
        if fields[0].is_empty() || fields[1].is_empty() {
            return Err(err(
                lineno,
                "a sankey row needs both a source and a target".to_string(),
            ));
        }
        let value: f64 = fields[2]
            .parse()
            .map_err(|_| err(lineno, format!("invalid sankey value: '{}'", fields[2])))?;
        if !value.is_finite() || value < 0.0 {
            return Err(err(
                lineno,
                format!("a sankey value must be finite and >= 0: '{}'", fields[2]),
            ));
        }
        let source_idx = d.ensure_node(&fields[0]);
        let target_idx = d.ensure_node(&fields[1]);
        d.links.push(SankeyLink {
            source: source_idx,
            target: target_idx,
            value,
        });
    }
    Ok(d)
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
    fn semicolon_one_liners_and_header_split() {
        // #19: the classic compact form from mermaid's README and
        // countless LLM/chat outputs — `;` on the HEADER line too.
        let g = parse("graph TD;A-->B;").unwrap();
        assert_eq!(g.direction, Direction::TD);
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        // Multiple statements sharing the header line.
        let g = parse("flowchart LR;A-->B;C-->D").unwrap();
        assert_eq!(g.direction, Direction::LR);
        assert_eq!(g.edges.len(), 2);
        // A `;` after a KEYWORD line: `subgraph two; A --> B` (the
        // issue's exact case) — the edge must not vanish into the id.
        let g = parse_document("flowchart TD\nsubgraph two; A --> B\nend\nC --> two").unwrap();
        let Document::Flowchart(g) = g else { panic!() };
        assert_eq!(g.subgraphs.len(), 1);
        assert_eq!(g.subgraphs[0].id, "two");
        assert_eq!(g.edges.len(), 1, "A --> B inside the block");
        assert_eq!(g.sub_edges.len(), 1, "C --> two binds the box");
        // A whole subgraph block on one line.
        let g = parse("flowchart TD\nsubgraph s; X; Y; end\nX --> Y").unwrap();
        assert_eq!(g.subgraphs[0].nodes.len(), 2);
    }

    #[test]
    fn semicolons_inside_labels_are_literal() {
        // `;` inside a quoted label, an edge label, or shape brackets
        // is text, not a separator.
        let g = parse("flowchart TD\nA[\"a; b\"] --> C").unwrap();
        assert_eq!(g.nodes.len(), 2, "quoted ';' must not split");
        assert_eq!(g.nodes[0].label, "a; b");
        let g = parse("flowchart TD\nA -->|\"go; stop\"| B").unwrap();
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].label.as_deref(), Some("go; stop"));
        let g = parse("flowchart TD\nA[a;b] --> C").unwrap();
        assert_eq!(g.nodes[0].label, "a;b", "bracketed ';' must not split");
        // A `;` in a comment line stays a comment (whole line dropped).
        let g = parse("flowchart TD\n%% one; two\nA --> B").unwrap();
        assert_eq!(g.nodes.len(), 2, "no phantom node from comment ';'");
        // A single literal `|` inside a shape label (`read|write`,
        // `Yes|No`) must NOT get stuck and swallow a later top-level
        // `;` — legal mermaid, common in practice.
        let g = parse("flowchart TD\nA[read|write]; B[x]").unwrap();
        assert_eq!(g.nodes.len(), 2, "bracketed '|' must not eat the ';'");
        assert_eq!(g.nodes[0].label, "read|write");
        let g = parse("flowchart TD\nA[Yes|No]; C[a|b|c]; D{e|f}").unwrap();
        assert_eq!(g.nodes.len(), 3, "odd and even bracketed pipes both fine");
        // A `;` right after a normal edge label still splits cleanly.
        let g = parse("flowchart TD\nA -->|go stop| B; C --> D").unwrap();
        assert_eq!(g.edges.len(), 2);
        assert_eq!(g.edges[0].label.as_deref(), Some("go stop"));
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
        // `gantt` is unsupported in both entry points.
        for res in [
            parse("gantt\ntitle x").map(|_| ()),
            parse_document("gantt\ntitle x").map(|_| ()),
        ] {
            let e = res.unwrap_err();
            assert_eq!(e.line, 1);
            assert!(
                e.message.contains("not supported yet"),
                "message should say the type is unsupported: {}",
                e.message
            );
        }
        // `timeline` IS parsed by parse_document, so the flowchart-only
        // `parse` must send the caller to the right entry point rather
        // than deny the type exists.
        let e = parse("timeline\nx").unwrap_err();
        assert!(e.message.contains("parse_document"), "{}", e.message);
        assert!(!e.message.contains("not supported yet"), "{}", e.message);
        assert!(matches!(
            parse_document("timeline\n2002 : A").unwrap(),
            Document::Timeline(_)
        ));
        // The flowchart-only `parse` refuses erDiagram with a pointer
        // to the right entry point.
        let e = parse("erDiagram\nA ||--o{ B : has").unwrap_err();
        assert!(e.message.contains("parse_document"), "{}", e.message);
        // A node that merely starts with a type name is still a node.
        let g = parse("pies[Pie Chart] --> B").unwrap();
        assert_eq!(g.nodes[0].id, "pies");
    }

    /// Locks the invariant that went stale once before: a header the
    /// crate CAN parse must never be reported as unsupported. Iterates
    /// [`DIAGRAM_HEADERS`] itself, so a new entry is covered without
    /// anyone remembering to extend this test.
    #[test]
    fn parse_sends_every_parsed_type_to_the_right_entry_point() {
        for &(header, sup) in DIAGRAM_HEADERS {
            if sup == HeaderSupport::Unsupported {
                continue;
            }
            let e = parse(&format!("{header}\n")).unwrap_err();
            assert!(
                e.message.contains("flowchart-only"),
                "'{header}' should point at parse_document, got: {}",
                e.message
            );
            assert!(
                !e.message.contains("not supported yet"),
                "'{header}' is parsed by the crate — the message must not deny it: {}",
                e.message
            );
        }
    }

    /// The other half of the lock. A `Parsed`/`Alias` entry with no arm
    /// in `parse_document` falls into the catch-all and reproduces the
    /// exact self-contradictory message this table exists to prevent —
    /// and the test above would still pass, because `parse` never
    /// reaches the dispatch. So assert the dispatch itself.
    #[test]
    fn parse_document_dispatches_every_parsed_header() {
        for &(header, sup) in DIAGRAM_HEADERS {
            let res = parse_document(&format!("{header}\n")).map(|_| ());
            if sup == HeaderSupport::Unsupported {
                let e = res.unwrap_err();
                assert!(
                    e.message.contains("not supported yet"),
                    "'{header}' has no parser and should say so: {}",
                    e.message
                );
                continue;
            }
            // An empty body may legitimately still be an error, but it
            // must never be "this diagram type does not exist".
            if let Err(e) = res {
                assert!(
                    !e.message.contains("not supported yet"),
                    "'{header}' is missing a dispatch arm: {}",
                    e.message
                );
            }
        }
    }

    #[test]
    fn supported_list_matches_the_dispatch_table() {
        let list = supported_types();
        assert!(list.contains("flowchart"), "{list}");
        assert!(list.contains("graph"), "{list}");
        for &(header, sup) in DIAGRAM_HEADERS {
            match sup {
                HeaderSupport::Parsed => {
                    assert!(list.contains(header), "'{header}' missing from: {list}");
                }
                HeaderSupport::Unsupported => {
                    assert!(
                        !list.contains(header),
                        "unparsed '{header}' leaked into: {list}"
                    );
                }
                // Matched, but deliberately not advertised a second time.
                HeaderSupport::Alias => {}
            }
        }
    }

    // --------------------------- Sankey ---------------------------

    fn sankey(src: &str) -> SankeyDiagram {
        match parse_document(src).unwrap() {
            Document::Sankey(d) => d,
            _ => panic!("not a sankey"),
        }
    }

    #[test]
    fn sankey_rows_build_nodes_in_first_mention_order() {
        let d = sankey("sankey-beta\nA,B,5\nB,C,3\nA,C,2\n");
        assert_eq!(d.nodes, vec!["A", "B", "C"]);
        assert_eq!(d.links.len(), 3);
        assert_eq!(d.links[0].source, 0);
        assert_eq!(d.links[0].target, 1);
        assert_eq!(d.links[0].value, 5.0);
    }

    #[test]
    fn sankey_accepts_both_header_spellings() {
        assert_eq!(sankey("sankey-beta\nA,B,1\n").links.len(), 1);
        assert_eq!(sankey("sankey\nA,B,1\n").links.len(), 1);
    }

    #[test]
    fn sankey_quoted_fields_keep_commas_and_escaped_quotes() {
        let d = sankey("sankey-beta\n\"Oil, crude\",\"He said \"\"hi\"\"\",7\n");
        assert_eq!(d.nodes[0], "Oil, crude");
        assert_eq!(d.nodes[1], "He said \"hi\"");
        assert_eq!(d.links[0].value, 7.0);
    }

    #[test]
    fn sankey_comments_and_blank_lines_are_skipped() {
        let d = sankey("sankey-beta\n\n%% a comment\nA,B,1\n\n");
        assert_eq!(d.links.len(), 1);
    }

    #[test]
    fn sankey_repeated_label_reuses_one_node() {
        let d = sankey("sankey-beta\nA,B,1\nA,B,2\n");
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.links.len(), 2);
    }

    #[test]
    fn sankey_row_shape_errors_carry_line_numbers() {
        let e = parse_document("sankey-beta\nA,B\n")
            .map(|_| ())
            .unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("source,target,value"), "{}", e.message);

        let e = parse_document("sankey-beta\nA,B,C,1\n")
            .map(|_| ())
            .unwrap_err();
        assert!(e.message.contains("4 fields"), "{}", e.message);

        let e = parse_document("sankey-beta\nA,B,xyz\n")
            .map(|_| ())
            .unwrap_err();
        assert!(e.message.contains("invalid sankey value"), "{}", e.message);

        let e = parse_document("sankey-beta\nA,B,-3\n")
            .map(|_| ())
            .unwrap_err();
        assert!(e.message.contains(">= 0"), "{}", e.message);

        let e = parse_document("sankey-beta\n,B,1\n")
            .map(|_| ())
            .unwrap_err();
        assert!(e.message.contains("source and a target"), "{}", e.message);

        let e = parse_document("sankey-beta\n\"A,B,1\n")
            .map(|_| ())
            .unwrap_err();
        assert!(e.message.contains("unclosed"), "{}", e.message);
    }

    #[test]
    fn sankey_defaults_match_mermaid_when_there_is_no_config() {
        let d = sankey("sankey-beta\nA,B,1\n");
        assert_eq!(d.config, SankeyConfig::default());
        assert_eq!(d.config.width, 600.0);
        assert_eq!(d.config.height, 400.0);
        assert_eq!(d.config.node_width, 10.0);
        assert_eq!(d.config.node_padding, 12.0);
        assert!(d.config.show_values);
        assert_eq!(d.config.link_color, SankeyLinkColor::Gradient);
        assert_eq!(d.config.node_alignment, SankeyAlignment::Justify);
    }

    #[test]
    fn sankey_reads_config_from_frontmatter() {
        let d = sankey(
            "---\nconfig:\n  sankey:\n    showValues: false\n    linkColor: source\n    \
             nodeAlignment: left\n    nodeWidth: 20\n    suffix: \" TWh\"\n---\nsankey-beta\nA,B,1\n",
        );
        assert!(!d.config.show_values);
        assert_eq!(d.config.link_color, SankeyLinkColor::Source);
        assert_eq!(d.config.node_alignment, SankeyAlignment::Left);
        assert_eq!(d.config.node_width, 20.0);
        assert_eq!(d.config.suffix, " TWh");
        // Untouched keys keep mermaid's defaults.
        assert_eq!(d.config.height, 400.0);
    }

    #[test]
    fn sankey_config_ignores_other_blocks_and_unknown_keys() {
        let d = sankey(
            "---\ntitle: Energy\nconfig:\n  theme: dark\n  pie:\n    useWidth: 9\n  \
             sankey:\n    nodeWidth: 4\n    somethingNew: 1\n---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(d.config.node_width, 4.0);
        // `pie`'s useWidth must not leak into sankey's width.
        assert_eq!(d.config.width, 600.0);
    }

    #[test]
    fn sankey_node_colors_read_block_and_flow_maps() {
        // mermaid 12's `nodeColors` (label -> colour) was ignored.
        let d = sankey(
            "---\nconfig:\n  sankey:\n    nodeColors:\n      Coal: \"#333333\"\n      \
             \"Industry, light\": '#ff8800'\n      width: \"#123456\"\n    nodeWidth: 4\n\
             ---\nsankey-beta\nCoal,width,1\n",
        );
        assert_eq!(
            d.config.node_colors,
            [
                ("Coal".to_string(), "#333333".to_string()),
                ("Industry, light".to_string(), "#ff8800".to_string()),
                ("width".to_string(), "#123456".to_string()),
            ]
        );
        // A node NAMED like a config key stays a colour entry, and the
        // block ends where the indentation returns to sankey's keys.
        assert_eq!(d.config.width, 600.0);
        assert_eq!(d.config.node_width, 4.0);

        let d = sankey(
            "---\nconfig:\n  sankey:\n    nodeColors: { A: red, \"B, c\": \"#00f\", D: null }\n\
             ---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(
            d.config.node_colors,
            [
                ("A".to_string(), "red".to_string()),
                ("B, c".to_string(), "#00f".to_string()),
            ],
            "a null colour keeps the palette"
        );
    }

    #[test]
    fn sankey_label_style_reads_outlined() {
        let d =
            sankey("---\nconfig:\n  sankey:\n    labelStyle: Outlined\n---\nsankey-beta\nA,B,1\n");
        assert_eq!(d.config.label_style, SankeyLabelStyle::Outlined);
        let d = sankey("---\nconfig:\n  sankey:\n    labelStyle: fancy\n---\nsankey-beta\nA,B,1\n");
        assert_eq!(d.config.label_style, SankeyLabelStyle::Legacy);
    }

    #[test]
    fn sankey_config_survives_a_bad_value() {
        // Config is decoration: a junk number keeps the default rather
        // than failing the whole diagram.
        let d = sankey("---\nconfig:\n  sankey:\n    nodeWidth: wide\n---\nsankey-beta\nA,B,1\n");
        assert_eq!(d.config.node_width, 10.0);
    }

    #[test]
    fn sankey_linkcolor_accepts_a_css_colour() {
        let d = sankey(
            "---\nconfig:\n  sankey:\n    linkColor: \"#ff0000\"\n---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(
            d.config.link_color,
            SankeyLinkColor::Fixed("#ff0000".to_string())
        );
    }

    #[test]
    fn sankey_showvalues_accepts_yaml_booleans() {
        // YAML 1.1 spells false several ways; matching only the
        // lowercase word left the setting silently switched on.
        for word in ["false", "False", "FALSE", "no", "off", "n", "0"] {
            let d = sankey(&format!(
                "---\nconfig:\n  sankey:\n    showValues: {word}\n---\nsankey-beta\nA,B,1\n"
            ));
            assert!(!d.config.show_values, "'{word}' should read as false");
        }
        for word in ["true", "True", "yes", "on", "1"] {
            let d = sankey(&format!(
                "---\nconfig:\n  sankey:\n    showValues: {word}\n---\nsankey-beta\nA,B,1\n"
            ));
            assert!(d.config.show_values, "'{word}' should read as true");
        }
        // An unrecognised word keeps the default rather than flipping it.
        let d = sankey("---\nconfig:\n  sankey:\n    showValues: maybe\n---\nsankey-beta\nA,B,1\n");
        assert!(d.config.show_values);
    }

    #[test]
    fn sankey_quoted_config_value_drops_a_trailing_comment() {
        let d = sankey(
            "---\nconfig:\n  sankey:\n    suffix: \" TWh\" # units\n---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(d.config.suffix, " TWh");
        // A `#` INSIDE the quotes belongs to the value, not to a comment.
        let d = sankey("---\nconfig:\n  sankey:\n    prefix: \"# \"\n---\nsankey-beta\nA,B,1\n");
        assert_eq!(d.config.prefix, "# ");
    }

    #[test]
    fn sankey_rejects_junk_after_a_closing_quote() {
        // Silently eating it would let a typo change which node a flow
        // attaches to.
        let e = parse_document("sankey-beta\n\"A\"junk,B,1\n")
            .map(|_| ())
            .unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("after a closing"), "{}", e.message);
        // Padding is still fine.
        let d = sankey("sankey-beta\n\"A\"  ,B,1\n");
        assert_eq!(d.nodes[0], "A");
    }

    #[test]
    fn quoted_css_url_values_are_accepted() {
        // Rejecting quotes at the parser was a first attempt at the
        // injection fix; it cost these, which are valid CSS. Escaping at
        // the render site protects without turning input away.
        for v in ["url('#g')", "url(\"#g\")", "url(#a-->b)", "rgb(1, 2, 3)"] {
            let src = format!("flowchart TD\n  A[Hi] --> B\n  style A fill:{v}\n");
            let g = parse(&src).unwrap_or_else(|e| panic!("'{v}' rejected: {}", e.message));
            assert_eq!(g.nodes[0].style.fill.as_deref(), Some(v));
        }
    }

    #[test]
    fn sankey_config_keywords_ignore_case() {
        // `showValues` was already case-insensitive; the two enum keys
        // beside it were not, so `Gradient` became a CSS colour named
        // "Gradient" and painted the ribbons black.
        for word in ["gradient", "Gradient", "GRADIENT"] {
            let d = sankey(&format!(
                "---\nconfig:\n  sankey:\n    linkColor: {word}\n---\nsankey-beta\nA,B,1\n"
            ));
            assert_eq!(d.config.link_color, SankeyLinkColor::Gradient, "{word}");
        }
        for word in ["left", "Left", "LEFT"] {
            let d = sankey(&format!(
                "---\nconfig:\n  sankey:\n    nodeAlignment: {word}\n---\nsankey-beta\nA,B,1\n"
            ));
            assert_eq!(d.config.node_alignment, SankeyAlignment::Left, "{word}");
        }
        // A real colour keeps its spelling — CSS `var(--Name)` is
        // case-sensitive.
        let d = sankey(
            "---\nconfig:\n  sankey:\n    linkColor: var(--Brand)\n---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(
            d.config.link_color,
            SankeyLinkColor::Fixed("var(--Brand)".to_string())
        );
    }

    #[test]
    fn sankey_config_rejects_non_finite_numbers() {
        // f64::from_str accepts these, and they reach the SVG as
        // `x="NaN"` / `viewBox="0 0 600 inf"`, which no renderer reads.
        for word in ["NaN", "inf", "-inf", "infinity", "-5"] {
            let d = sankey(&format!(
                "---\nconfig:\n  sankey:\n    nodeWidth: {word}\n    height: {word}\n---\nsankey-beta\nA,B,1\n"
            ));
            assert_eq!(d.config.node_width, 10.0, "nodeWidth kept '{word}'");
            assert_eq!(d.config.height, 400.0, "height kept '{word}'");
        }
    }

    #[test]
    fn sankey_config_only_reads_a_direct_child_of_config() {
        // A `sankey` key nested under another namespace must not
        // reconfigure the diagram.
        let d = sankey(
            "---\nconfig:\n  themeVariables:\n    sankey:\n      nodeWidth: 40\n---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(d.config.node_width, 10.0, "nested sankey block leaked");
        // The direct child still works.
        let d = sankey(
            "---\nconfig:\n  themeVariables:\n    x: 1\n  sankey:\n    nodeWidth: 40\n---\nsankey-beta\nA,B,1\n",
        );
        assert_eq!(d.config.node_width, 40.0);
    }

    #[test]
    fn mermaid_swimlane_header_fails_explicitly() {
        // Mermaid added `swimlane-beta` in v11.16. The catalogue had it
        // written as `swimlanes`, so the real token was never registered
        // and fell through to the flowchart parser, which complained
        // about the `-beta` suffix as if it were an edge operator.
        let e = parse_document("swimlane-beta\n  subgraph Alice\n  end\n")
            .map(|_| ())
            .unwrap_err();
        assert_eq!(e.line, 1);
        assert!(e.message.contains("not supported yet"), "{}", e.message);
        assert!(!e.message.contains("edge operator"), "{}", e.message);
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
        let d = er(
            "erDiagram\nT {\n  varchar(20) difficulty \"not null, default 'medium' (see docs)\"\n}",
        );
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

    /// Report P1: `direction` was a parse error ("unknown relationship
    /// operator near: 'TB'"), so the hard-coded flow could not be
    /// changed. Mermaid's default is top to bottom.
    #[test]
    fn er_direction_statement_parses() {
        assert_eq!(er("erDiagram\nA ||--o{ B").direction, Direction::TD);
        for (src, want) in [
            ("TB", Direction::TD),
            ("TD", Direction::TD),
            ("BT", Direction::BT),
            ("LR", Direction::LR),
            ("rl", Direction::RL),
        ] {
            let d = er(&format!("erDiagram\n  direction {src}\nA ||--o{{ B"));
            assert_eq!(d.direction, want, "{src}");
            assert_eq!(d.relations.len(), 1);
        }
        let e = parse_document("erDiagram\ndirection sideways\n").unwrap_err();
        assert_eq!(e.line, 2);
        // An entity may still be called `direction`.
        let d = er("erDiagram\ndirection ||--o{ B\ndirection {\n  int id\n}");
        assert_eq!(d.entities[0].name, "direction");
        assert_eq!(d.entities[0].attrs.len(), 1);
    }

    /// Report P1: `p[Person]` was rejected. Mermaid takes an alias on
    /// an entity declaration (bare or opening a block); flowmaid also
    /// takes it in a relationship. The first alias sticks, as in
    /// mermaid's `addEntity`.
    #[test]
    fn er_entity_alias_parses() {
        let d = er("erDiagram\n\
                    p[Person] {\n  string name\n}\n\
                    c[\"Customer Account\"]\n\
                    o[Order] ||--o{ i[Item] : has\n\
                    p ||--o{ c : owns\n\
                    p[Someone else]\n");
        let alias = |n: &str| {
            let e = d.entities.iter().find(|e| e.name == n).unwrap();
            (e.alias.clone(), e.title().to_string())
        };
        assert_eq!(alias("p"), (Some("Person".into()), "Person".into()));
        assert_eq!(alias("c").1, "Customer Account");
        assert_eq!(alias("o").1, "Order");
        assert_eq!(alias("i").1, "Item");
        assert_eq!(d.entities[0].attrs.len(), 1);
        assert_eq!(d.relations.len(), 2);
        let e = parse_document("erDiagram\np[Person ||--o{ B\n").unwrap_err();
        assert_eq!(e.line, 2);
    }

    #[test]
    fn er_quoted_entity_names_parse() {
        let d = er("erDiagram\n\"Customer Account\" ||--o{ ORDER : places\n\
                    \"Customer Account\" {\n  int id PK\n}\n");
        assert_eq!(d.entities[0].name, "Customer Account");
        assert_eq!(d.entities[0].attrs.len(), 1);
        assert_eq!(d.entities.len(), 2);
    }

    /// Mermaid tokenises an entity block: rows and the closing brace may
    /// share a line, several rows may share one, keys are any case.
    #[test]
    fn er_block_rows_tokenise_like_mermaid() {
        let d = er("erDiagram\n\
                    A { int id PK \"key\" string name }\n\
                    B {}\n\
                    C { int id pk, fk\n  string a string b \"c, d\"\n  text t }\n\
                    A ||--o{ B : has\n");
        let a = &d.entities[0];
        assert_eq!(a.attrs.len(), 2);
        assert_eq!(a.attrs[0].keys, vec![Key::Pk]);
        assert_eq!(a.attrs[0].comment.as_deref(), Some("key"));
        assert_eq!(a.attrs[1].name, "name");
        assert!(d.entities[1].attrs.is_empty());
        let c = &d.entities[2];
        let names: Vec<&str> = c.attrs.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["id", "a", "b", "t"]);
        assert_eq!(c.attrs[0].keys, vec![Key::Pk, Key::Fk]);
        assert_eq!(c.attrs[2].comment.as_deref(), Some("c, d"));
        assert_eq!(d.relations.len(), 1);
        // A comment holding quotes still reads as one classic row.
        let d = er("erDiagram\nT {\n  text a \"say \"hi\" twice\"\n}");
        assert_eq!(
            d.entities[0].attrs[0].comment.as_deref(),
            Some("say \"hi\" twice")
        );
        // A bad key is still named.
        let e = parse_document("erDiagram\nT {\n  int id XX\n}").unwrap_err();
        assert!(e.message.contains("'XX'"), "{}", e.message);
        let e = parse_document("erDiagram\nT { int id } extra\n").unwrap_err();
        assert_eq!(e.line, 2);
    }

    /// Mermaid's spelled-out cardinalities and lines, and any symbol on
    /// either side.
    #[test]
    fn er_text_cardinalities_parse() {
        let rel = |op: &str| {
            let d = er(&format!("erDiagram\nA {op} B : r"));
            let r = &d.relations[0];
            (r.card_from, r.identifying, r.card_to)
        };
        use Card::*;
        assert_eq!(rel("only one to zero or more"), (One, true, ZeroMany));
        assert_eq!(rel("one or more optionally to one"), (OneMany, false, One));
        assert_eq!(rel("zero or one to many(1)"), (ZeroOne, true, OneMany));
        assert_eq!(rel("1+ .. 0+"), (OneMany, false, ZeroMany));
        assert_eq!(rel("1 to many"), (One, true, ZeroMany));
        assert_eq!(rel("one or zero .- many(0)"), (ZeroOne, false, ZeroMany));
        assert_eq!(rel("|| -. |{"), (One, false, OneMany));
        assert_eq!(rel("o|--}o"), (ZeroOne, true, ZeroMany));
        assert_eq!(rel("ONLY ONE TO ZERO OR MANY"), (One, true, ZeroMany));
        // A word that only starts like a token is not one.
        assert!(parse_document("erDiagram\nA oneway to one B").is_err());
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
        assert!(parse("A --> B\nstyle A glow:heavy,fill:#fff")
            .unwrap()
            .nodes[0]
            .style
            .fill
            .is_some());
    }

    /// (kind, start glyph, resolved end glyph, label) of the one
    /// edge in `flowchart LR\nA <op> B`.
    fn one_edge(op: &str) -> (EdgeKind, Option<Marker>, Option<Marker>, Option<String>) {
        let g = parse(&format!("flowchart LR\nA {op} B\n"))
            .unwrap_or_else(|e| panic!("{op}: {}", e.message));
        assert_eq!(g.edges.len(), 1, "{op}");
        assert_eq!(g.node_index("B"), Some(1), "{op}: target is B");
        let e = &g.edges[0];
        (
            e.kind,
            e.ends.start_marker(e.kind),
            e.ends.end_marker(e.kind),
            e.label.clone(),
        )
    }

    #[test]
    fn cross_and_circle_end_markers_parse_on_every_line_style() {
        use EdgeKind::*;
        use Marker::{Circle, Cross};
        let l = |s: &str| Some(s.to_string());
        let cases: &[(&str, EdgeKind, Option<Marker>, Option<String>)] = &[
            ("--x", Open, Some(Cross), None),
            ("-.-x", DottedOpen, Some(Cross), None),
            ("==x", ThickOpen, Some(Cross), None),
            ("--o", Open, Some(Circle), None),
            ("-.-o", DottedOpen, Some(Circle), None),
            ("==o", ThickOpen, Some(Circle), None),
            // Longer links, as mermaid allows.
            ("----x", Open, Some(Cross), None),
            ("-...-o", DottedOpen, Some(Circle), None),
            ("====x", ThickOpen, Some(Cross), None),
            // Pipe labels.
            ("--x|l|", Open, Some(Cross), l("l")),
            ("-.-o|l|", DottedOpen, Some(Circle), l("l")),
            ("==x|l|", ThickOpen, Some(Cross), l("l")),
            // Inline labels.
            ("-- text --x", Open, Some(Cross), l("text")),
            ("-- text --o", Open, Some(Circle), l("text")),
            ("-. text .-x", DottedOpen, Some(Cross), l("text")),
            ("-. text .-o", DottedOpen, Some(Circle), l("text")),
            ("== text ==x", ThickOpen, Some(Cross), l("text")),
            ("== text ==o", ThickOpen, Some(Circle), l("text")),
            ("-- text ----x", Open, Some(Cross), l("text")),
        ];
        for (op, kind, end, label) in cases {
            assert_eq!(one_edge(op), (*kind, None, *end, label.clone()), "{op}");
        }
    }

    #[test]
    fn bidirectional_links_parse_with_glyphs_at_both_ends() {
        use EdgeKind::*;
        use Marker::{Circle, Cross};
        let l = |s: &str| Some(s.to_string());
        let cases: &[(&str, EdgeKind, Marker, Option<String>)] = &[
            ("<-->", Arrow, Marker::Arrow, None),
            ("<-.->", Dotted, Marker::Arrow, None),
            ("<==>", Thick, Marker::Arrow, None),
            ("x--x", Open, Cross, None),
            ("o--o", Open, Circle, None),
            ("x-.-x", DottedOpen, Cross, None),
            ("o==o", ThickOpen, Circle, None),
            ("<---->", Arrow, Marker::Arrow, None),
            ("<-..->", Dotted, Marker::Arrow, None),
            ("<===>", Thick, Marker::Arrow, None),
            ("<-->|l|", Arrow, Marker::Arrow, l("l")),
            ("x--x|l|", Open, Cross, l("l")),
            ("<-- text -->", Arrow, Marker::Arrow, l("text")),
            ("<-. text .->", Dotted, Marker::Arrow, l("text")),
            ("<== text ==>", Thick, Marker::Arrow, l("text")),
            ("x-- text --x", Open, Cross, l("text")),
            ("o-. text .-o", DottedOpen, Circle, l("text")),
        ];
        for (op, kind, m, label) in cases {
            assert_eq!(
                one_edge(op),
                (*kind, Some(*m), Some(*m), label.clone()),
                "{op}"
            );
        }
        // Mermaid only honours a start glyph that mirrors the end one;
        // a plain link with a mismatched lead is drawn one-way…
        assert_eq!(one_edge("<---"), (Open, None, None, None));
        assert_eq!(one_edge("x-->"), (Arrow, None, Some(Marker::Arrow), None));
        // …and a labelled one is rejected, as in mermaid.
        let e = parse("flowchart LR\nA <-- t ---x B\n").unwrap_err();
        assert!(e.message.contains("matching end"), "{}", e.message);
    }

    #[test]
    fn marker_operators_no_longer_read_as_unclosed_inline_labels() {
        for op in ["--x", "--o", "==x", "==o", "-.-x", "-.-o"] {
            for src in [
                format!("flowchart LR\nA {op} B\n"),
                format!("flowchart LR\nA{op}B\n"),
                format!("flowchart LR\nA {op} B --> C\n"),
            ] {
                let g = parse(&src).unwrap_or_else(|e| panic!("{src:?}: {}", e.message));
                assert!(g.edges[0].ends.end.is_some(), "{src:?}");
                assert_eq!(g.nodes[1].id, "B", "{src:?}");
            }
        }
        // The old diagnostic still fires for a genuinely open label.
        let e = parse("flowchart LR\nA -- text B\n").unwrap_err();
        assert!(e.message.contains("never closed"), "{}", e.message);
    }

    #[test]
    fn node_ids_starting_with_x_or_o_still_work() {
        let ids = |src: &str| {
            let g = parse(src).unwrap_or_else(|e| panic!("{src:?}: {}", e.message));
            let v: Vec<(String, String, EdgeKind, EdgeEnds)> = g
                .edges
                .iter()
                .map(|e| {
                    (
                        g.nodes[e.from].id.clone(),
                        g.nodes[e.to].id.clone(),
                        e.kind,
                        e.ends,
                    )
                })
                .collect();
            v
        };
        let plain = EdgeEnds::default();
        let edge = |a: &str, b: &str, k, e| (a.to_string(), b.to_string(), k, e);
        assert_eq!(
            ids("flowchart LR\nA --> xray\n"),
            [edge("A", "xray", EdgeKind::Arrow, plain)]
        );
        assert_eq!(
            ids("flowchart LR\nA --- ok\n"),
            [edge("A", "ok", EdgeKind::Open, plain)]
        );
        assert_eq!(
            ids("flowchart LR\nx --> o\n"),
            [edge("x", "o", EdgeKind::Arrow, plain)]
        );
        assert_eq!(
            ids("flowchart LR\nA -- ok --> B\n"),
            [edge("A", "B", EdgeKind::Arrow, plain)]
        );
        assert_eq!(
            ids("flowchart LR\nA -->oB\nox --x xo\n"),
            [
                edge("A", "oB", EdgeKind::Arrow, plain),
                edge(
                    "ox",
                    "xo",
                    EdgeKind::Open,
                    EdgeEnds {
                        start: None,
                        end: Some(Marker::Cross)
                    }
                ),
            ]
        );
        // A start character glued to the source id is part of the id
        // (mermaid's NODE_STRING eats it): `Ax--xB` is Ax → B.
        assert_eq!(ids("flowchart LR\nAx--xB\n")[0].0, "Ax");
        // Mermaid's documented pitfall: a marker letter glued to the
        // operator is the marker, so `A---oB` is a circle edge to B.
        assert_eq!(
            ids("flowchart LR\nA---oB\n"),
            [edge(
                "A",
                "B",
                EdgeKind::Open,
                EdgeEnds {
                    start: None,
                    end: Some(Marker::Circle)
                }
            )]
        );
    }

    #[test]
    fn quoted_inline_labels_may_hold_closer_text() {
        // Mermaid's string state shields quoted text, so a closer
        // inside the quotes never ends the label.
        for (line, label) in [
            ("A -- \"use --xml\" --> B", "use --xml"),
            ("A -- \"a --o b\" --> B", "a --o b"),
            ("A == \"a ==x\" ==> B", "a ==x"),
            ("A -- \"x--x\" --> B", "x--x"),
            ("A -- \"go--on\" --> B", "go--on"),
            ("A -. \"a .-x\" .-> B", "a .-x"),
            ("A -- \"a --> b\" --> B", "a --> b"),
            ("A x-- \"t --x\" --x B", "t --x"),
        ] {
            let g = parse(&format!("flowchart LR\n{line}\n"))
                .unwrap_or_else(|e| panic!("{line:?}: {}", e.message));
            assert_eq!(g.edges.len(), 1, "{line:?}");
            assert_eq!(g.nodes[g.edges[0].to].id, "B", "{line:?}");
            assert_eq!(g.edges[0].label.as_deref(), Some(label), "{line:?}");
        }
        // The keyword gate skips quotes the same way.
        let g = parse("flowchart LR\nclick -- \"a --x b\" --> B\n").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("a --x b"));
    }

    /// (from, to, label, start marker, end marker) of each node edge.
    type EdgeRow = (
        String,
        String,
        Option<String>,
        Option<Marker>,
        Option<Marker>,
    );

    fn edge_rows(src: &str) -> Vec<EdgeRow> {
        let g = parse(src).unwrap_or_else(|e| panic!("{src:?}: {}", e.message));
        g.edges
            .iter()
            .map(|e| {
                (
                    g.nodes[e.from].id.clone(),
                    g.nodes[e.to].id.clone(),
                    e.label.clone(),
                    e.ends.start,
                    e.ends.end,
                )
            })
            .collect()
    }

    fn row(
        a: &str,
        b: &str,
        l: Option<&str>,
        start: Option<Marker>,
        end: Option<Marker>,
    ) -> EdgeRow {
        (a.into(), b.into(), l.map(String::from), start, end)
    }

    #[test]
    fn unquoted_inline_labels_end_at_the_first_marker_closer() {
        // Mermaid's lexer tries its LINK rule `\s*[xo<]?--+[-xo>]` /
        // `\s*[xo<]?==+[=xo>]` before every edge-text character, so
        // edge text closes at the first match, even mid-word, and an
        // `x` / `o` / `<` right before the closer belongs to it.
        let circle = Some(Marker::Circle);
        let cross = Some(Marker::Cross);
        // `o--o` mirrors itself: a two-way circle edge labelled `g`.
        assert_eq!(
            edge_rows("flowchart LR\nA -- go--on --> B\n"),
            [
                row("A", "n", Some("g"), circle, circle),
                row("n", "B", None, None, None)
            ]
        );
        assert_eq!(
            edge_rows("flowchart LR\nA == go==on ==> B\n"),
            [
                row("A", "n", Some("g"), circle, circle),
                row("n", "B", None, None, None)
            ]
        );
        // `==o` alone: `i` is not a start character.
        assert_eq!(
            edge_rows("flowchart LR\nA == hi==on ==> B\n"),
            [
                row("A", "n", Some("hi"), None, circle),
                row("n", "B", None, None, None)
            ]
        );
        // `o--x` doesn't mirror: the `o` is eaten, the end is a cross.
        assert_eq!(
            edge_rows("flowchart LR\nA -- hello--x B\n"),
            [row("A", "B", Some("hell"), None, cross)]
        );
        // `<---` has no arrowhead to mirror: a plain line.
        assert_eq!(
            edge_rows("flowchart LR\nA -- a <--- B\n"),
            [row("A", "B", Some("a"), None, None)]
        );
        // A mirroring closer after a start character is invalid there.
        assert!(parse("flowchart LR\nA x-- t x--x B\n").is_err());
        assert_eq!(
            edge_rows("flowchart LR\nA --xfer--> B\n"),
            [
                row("A", "fer", None, None, cross),
                row("fer", "B", None, None, None)
            ]
        );
        // Closed right before another operator: no target, an error.
        for line in [
            "A -- to--x --> B",
            "A --o--> B",
            "A -- use --xml flag --> B",
        ] {
            assert!(
                parse(&format!("flowchart LR\n{line}\n")).is_err(),
                "{line:?}"
            );
        }
    }

    #[test]
    fn dotted_inline_labels_close_at_any_dotted_closer() {
        // Mermaid's dotted LINK rule is `[xo<]?-?.+-[xo>]?`.
        let cross = Some(Marker::Cross);
        for line in [
            "A -. t -.-x B",
            "A -. t ..-x B",
            "A -. t .-x B",
            "A -. t -..-x B",
        ] {
            assert_eq!(
                edge_rows(&format!("flowchart LR\n{line}\n")),
                [row("A", "B", Some("t"), None, cross)],
                "{line:?}"
            );
        }
        let g = parse("flowchart LR\nA -. t -.-> B\nC -. u ..- D\n").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("t"));
        assert_eq!(g.edges[0].kind, EdgeKind::Dotted);
        assert_eq!(g.edges[1].label.as_deref(), Some("u"));
        assert_eq!(g.edges[1].kind, EdgeKind::DottedOpen);
    }

    #[test]
    fn keyword_heads_with_marker_operators() {
        // A labelled two-way / marker edge from a keyword-named node
        // stays an edge…
        for line in [
            "click x-- t --x B",
            "linkStyle <-- t --> B",
            "click o-- t --o B",
        ] {
            let g = parse(&format!("flowchart LR\n{line}\n"))
                .unwrap_or_else(|e| panic!("{line:?}: {}", e.message));
            assert_eq!(g.edges.len(), 1, "{line:?}");
            assert!(g.edges[0].ends.start.is_some(), "{line:?}");
        }
        // …but an operator with nothing after it is not an edge, so it
        // stays a (malformed) click statement.
        let e = parse("flowchart LR\nA --> B\nclick x--x\n").unwrap_err();
        assert!(
            e.message
                .contains("click: expected a url, callback or 'call'"),
            "{}",
            e.message
        );
        // An unclosed two-way opener names the whole opener.
        let e = parse("flowchart LR\nA <-- B\n").unwrap_err();
        assert!(e.message.contains("'<--' is never closed"), "{}", e.message);
    }

    #[test]
    fn marker_edges_reach_subgraph_endpoints() {
        let g = parse("flowchart LR\nsubgraph S\nA\nend\nB <--> S\nS --x C\n").unwrap();
        assert_eq!(g.sub_edges.len(), 2);
        assert_eq!(g.sub_edges[0].ends.start, Some(Marker::Arrow));
        assert_eq!(g.sub_edges[1].ends.end, Some(Marker::Cross));
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
            vis.width,
            vis.height,
            inv.width,
            inv.height
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
        assert!(
            !g.subgraphs[0].nodes.contains(&w1),
            "and backend lets it go"
        );
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
        // Edge labels are single-line; subgraph titles keep their
        // `<br/>` lines, as mermaid draws them.
        let g = parse("A -->|one<br/>two| B\nsubgraph s[\"Ti<br/>tle\"]\nB\nend").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("one two"));
        assert_eq!(g.subgraphs[0].title, "Ti\ntle");
        // Empty subgraph: scene and route agree (no phantom cluster).
        let g = parse("flowchart TD\nA-->B\nsubgraph empty\nend\nB --> empty").unwrap();
        let s = crate::scene::scene(&g);
        let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = crate::scene::route(&g, &pos);
        assert_eq!(
            s.clusters.len(),
            r.clusters.len(),
            "cluster count consistent"
        );
        assert_eq!(s.edges.len(), r.edges.len(), "edge count consistent");
    }

    #[test]
    fn br_becomes_newline_and_grows_the_node() {
        let g =
            parse("flowchart TD\nA[\"CloudFront<br/>E2GQ<br />dbfu9k\"] --> B[One line]").unwrap();
        assert_eq!(g.nodes[0].label, "CloudFront\nE2GQ\ndbfu9k");
        // <BR> uppercase and bare <br> also work.
        let g2 = parse("A[x<BR>y<br>z]").unwrap();
        assert_eq!(g2.nodes[0].label, "x\ny\nz");
        // Taller node for the multi-line label; wider = widest line.
        let (w3, h3) = crate::layout::intrinsic_size(&g.nodes[0]);
        let (_w1, h1) = crate::layout::intrinsic_size(&g.nodes[1]);
        // Two extra lines add ~2×LINE_H of height.
        assert!(
            h3 > h1 + 30.0,
            "3-line node taller by ~2 lines: {h3} vs {h1}"
        );
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
        assert_eq!(
            shapes.iter().filter(|s| **s == Shape::StateStart).count(),
            2
        );
        assert_eq!(shapes.iter().filter(|s| **s == Shape::StateEnd).count(), 2);
        // Composite membership: its [*]s and Inner belong to B.
        assert_eq!(g.subgraphs.len(), 1);
        assert_eq!(g.subgraphs[0].nodes.len(), 3, "start + Inner + end");
        // Pseudostates draw no text.
        assert!(g
            .nodes
            .iter()
            .filter(|n| n.shape == Shape::StateStart)
            .all(|n| n.label.is_empty()));
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
        let g = state("stateDiagram-v2\nA --> Grp : go\nstate Grp {\n[*] --> X\n}\ndirection LR");
        assert_eq!(g.sub_edges.len(), 1, "forward ref to the composite box");
        assert!(matches!(g.sub_edges[0].to, End::Sub(0)));
        assert_eq!(g.sub_edges[0].label.as_deref(), Some("go"));
        assert_eq!(g.direction, crate::model::Direction::LR);
    }

    #[test]
    fn state_notes_become_cards_and_v1_header_works() {
        let g = state(
            "stateDiagram\nA --> B\nnote right of A : hi <br/> there\n\
             note left of B\nmulti\n  line\nEND NOTE\nB --> A",
        );
        // Two states, two note cards; two transitions, two note links.
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(g.edges.len(), 4);
        let card = |text: &str| {
            let v = g.nodes.iter().position(|n| n.label == text).unwrap();
            let n = &g.nodes[v];
            assert!(n.id.starts_with("__note_"), "{}", n.id);
            assert_eq!(n.shape, Shape::Rect);
            let note = crate::style::NOTE_STYLE;
            assert_eq!(n.style.fill.as_deref(), Some(note.fill));
            assert_eq!(n.style.stroke.as_deref(), Some(note.stroke));
            v
        };
        let (a, b) = (g.node_index("A").unwrap(), g.node_index("B").unwrap());
        let link = |from: usize, to: usize| {
            let e = g.edges.iter().find(|e| e.from == from && e.to == to);
            let e = e.expect("a note link");
            assert_eq!((e.kind, e.label.as_deref()), (EdgeKind::DottedOpen, None));
        };
        // `right of`: the state links to its card (after it on the flow);
        // `left of`: the card links to the state (before it).
        let hi = card("hi\nthere");
        link(a, hi);
        let multi = card("multi\nline");
        link(multi, b);
        assert_eq!(g.lead_nodes, vec![multi]);
    }

    #[test]
    fn state_notes_join_their_states_box() {
        let g = state(
            "stateDiagram-v2\nstate Outer {\nX --> Y\nnote right of X : in\n}\n\
             note left of Y : from outside\nnote right of Outer : on the box\n\
             state Later {\nZ --> W\n}\nnote left of Later : ahead",
        );
        let at = |text: &str| g.nodes.iter().position(|n| n.label == text).unwrap();
        // A note on a member sits in the member's box, wherever the
        // note line is.
        let members = &g.subgraphs[0].nodes;
        assert!(members.contains(&at("in")) && members.contains(&at("from outside")));
        // A note on a composite links to its frame from the parent box.
        let boxed = at("on the box");
        assert!(g.subgraphs.iter().all(|s| !s.nodes.contains(&boxed)));
        let sub = |v: usize| {
            g.sub_edges
                .iter()
                .find(|e| e.from == End::Node(v) || e.to == End::Node(v))
                .expect("a note link to a frame")
        };
        let e = sub(boxed);
        assert_eq!((e.from, e.to), (End::Sub(0), End::Node(boxed)));
        assert_eq!(e.kind, EdgeKind::DottedOpen);
        let ahead = at("ahead");
        let e = sub(ahead);
        assert_eq!((e.from, e.to), (End::Node(ahead), End::Sub(1)));
        assert!(g.lead_nodes.contains(&ahead));
    }

    #[test]
    fn state_note_forms_mermaid_ignores_stay_harmless() {
        // A floating note used to open a block that swallowed the rest
        // of the diagram; it is now a free card named by its alias.
        let g = state("stateDiagram-v2\nnote \"free text\" as N1\nA --> B\nN1 --> A");
        let n1 = &g.nodes[g.node_index("N1").unwrap()];
        assert_eq!((n1.label.as_str(), n1.shape), ("free text", Shape::Rect));
        assert!(n1.style.fill.is_some());
        assert_eq!(g.edges.len(), 2, "the lines after it still parse");
        // Note forms with no state to attach to are skipped as before,
        // a block one up to its `end note`.
        let g =
            state("stateDiagram-v2\nnote over A : x\nnote over A\nA --> Gone\nend note\nA --> B");
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        // A block note never closed keeps what it swallowed, visibly.
        let g = state("stateDiagram-v2\nA --> B\nnote right of B\nB --> C");
        assert!(g.nodes.iter().any(|n| n.label == "B --> C"));
        assert!(g.node_index("C").is_none());
        // Card ids never collide with a state the source already named.
        let g = state("stateDiagram-v2\n__note_1 --> X\nnote right of X : n");
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.nodes[g.node_index("__note_1").unwrap()].label, "__note_1");
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
    fn yaml_frontmatter_is_accepted_and_dropped_for_every_diagram_type() {
        // mermaid-live / GitHub exports lead with a `---` block (#20).
        for body in [
            "flowchart TD\nA --> B",
            "erDiagram\nA ||--o{ B : has",
            "classDiagram\nAnimal <|-- Dog",
            "sequenceDiagram\nA->>B: hi",
            "pie\n\"a\" : 1",
            "stateDiagram-v2\n[*] --> A",
            "mindmap\n  root\n    child",
            "journey\n  title T\n  section S\n    Task: 5: Me",
        ] {
            let fm = format!("---\ntitle: My Diagram\nconfig:\n  theme: dark\n---\n{body}");
            assert!(
                parse_document(&fm).is_ok(),
                "frontmatter must not break: {body}"
            );
        }
        // BOM + frontmatter together.
        assert!(parse_document("\u{feff}---\ntitle: X\n---\nflowchart TD\nA --> B").is_ok());
        // The flowchart-only entry point too.
        assert!(parse("---\ntitle: X\n---\nA --> B").is_ok());

        // Fence lines carry an indent in mermaid; open and close must
        // SHARE it, so a differently-indented `---` inside a `title:|`
        // block scalar is body, not an early close (regression fix).
        let g = parse("---\ntitle: |\n  first\n  ---\n  second\nkey: v\n---\nA --> B").unwrap();
        assert_eq!(
            g.nodes.len(),
            2,
            "indented '---' must not close the block early"
        );
        // A `...` document-end marker inside the body is likewise not
        // a fence (only `---` closes).
        let g = parse("---\ntitle: |\n  ...\n  more\n---\nA --> B").unwrap();
        assert_eq!(g.nodes.len(), 2, "'...' must not close the block early");
        // An indented open fence is honoured, and its close must match
        // that same indent.
        assert!(parse("  ---\n  title: X\n  ---\nA --> B").is_ok());

        // Frontmatter is stripped EXACTLY once: parse_document must
        // not re-strip inside its flowchart arm (a body `---` line is
        // a flowchart statement, not a second frontmatter block).
        let with_fm = parse_document("---\ntitle: X\n---\nflowchart TD\nA --> B").unwrap();
        let plain = parse_document("flowchart TD\nA --> B").unwrap();
        let ids = |d: &Document| match d {
            Document::Flowchart(g) => g.nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>(),
            _ => panic!(),
        };
        assert_eq!(
            ids(&with_fm),
            ids(&plain),
            "frontmatter must not alter the body"
        );
    }

    #[test]
    fn frontmatter_preserves_line_numbers_and_rejects_when_unterminated() {
        // An error in the body still points at the true source line.
        let src = "---\ntitle: X\n---\nflowchart TD\nstyle\n";
        let e = parse_document(src).unwrap_err();
        assert_eq!(
            e.line, 5,
            "line number must survive the frontmatter blanking"
        );
        // A `---` that never closes is a clean error, not silent junk.
        let e = parse_document("---\ntitle: X\nflowchart TD\nA --> B").unwrap_err();
        assert!(
            e.message.contains("unterminated frontmatter"),
            "{}",
            e.message
        );
        // An indented `---` does NOT satisfy the (un-indented) close,
        // so a block with only that is still unterminated — flowmaid
        // errors where a silent success would corrupt the diagram.
        let e = parse("---\ntitle: |\n  ...\n  ---\nA --> B").unwrap_err();
        assert!(e.message.contains("unterminated"), "{}", e.message);
        // A bare `---` mid-diagram is NOT frontmatter (only the FIRST
        // line opens a block), and an open link `A --- B` is never a
        // fence.
        assert!(parse("flowchart TD\nA --- B").is_ok());
        // Frontmatter must be the very first line — a leading blank
        // means no frontmatter (matches mermaid's `^` anchor), so the
        // stray `---` on line 2 is not stripped.
        assert!(parse_document("\n---\ntitle: X\n---\nflowchart TD\nA --> B").is_err());
    }

    #[test]
    fn fence_indent_recognizes_only_true_fences() {
        // Exactly three dashes, leading spaces/tabs and trailing
        // whitespace allowed; the returned indent is the leading run.
        assert_eq!(fence_indent("---"), Some(""));
        assert_eq!(fence_indent("---   "), Some(""));
        assert_eq!(fence_indent("  ---"), Some("  "));
        assert_eq!(fence_indent("\t ---  \t"), Some("\t "));
        // Not fences: wrong dash count, content after the dashes, an
        // open-link edge, or a non-fence line.
        assert_eq!(fence_indent("----"), None);
        assert_eq!(fence_indent("--"), None);
        assert_eq!(fence_indent("--- x"), None);
        assert_eq!(fence_indent("A --- B"), None);
        assert_eq!(fence_indent("title: X"), None);
        assert_eq!(fence_indent(""), None);
    }

    #[test]
    fn frontmatter_edge_cases_are_robust() {
        // CRLF line endings (Windows / GitHub exports).
        assert!(parse_document("---\r\ntitle: X\r\n---\r\nflowchart TD\r\nA --> B").is_ok());
        // Trailing whitespace on the fences themselves.
        assert!(parse("---  \ntitle: X\n---\t\nA --> B").is_ok());
        // Multi-key config block, and a body line that itself looks
        // like an edge operator must not confuse the fence scan.
        assert!(parse_document(
            "---\ntitle: My Flow\nconfig:\n  theme: dark\n  flowchart:\n    curve: basis\n---\nflowchart LR\nA --> B"
        )
        .is_ok());
        // Mismatched-indent close: open at col 0, only an indented
        // `---` later → never closes → unterminated (not a silent
        // early close).
        assert!(parse("---\nx: 1\n  ---\nA --> B").is_err());
        // Open indented, close at col 0 → indents differ → unterminated.
        assert!(parse("  ---\nx: 1\n---\nA --> B").is_err());
        // Frontmatter directly followed by the header, no body keys.
        assert!(parse_document("---\n---\nflowchart TD\nA --> B").is_ok());
        // A `%%` comment as the first line means NO frontmatter (the
        // fence isn't the first line), so a later bare `---` is body.
        assert!(parse_document("%% note\n---\nA --> B").is_err());

        // The dropped block never leaks into node/edge content: the
        // graph equals the same diagram with no frontmatter.
        let fm = parse("---\ntitle: Ignore Me\nauthor: nobody\n---\nA[Start] --> B[End]").unwrap();
        let plain = parse("A[Start] --> B[End]").unwrap();
        assert_eq!(fm.nodes.len(), plain.nodes.len());
        assert_eq!(fm.nodes[0].label, "Start");
        assert_eq!(fm.nodes[1].label, "End");
        assert_eq!(fm.edges.len(), 1);

        // Line numbers stay exact under a MULTI-line frontmatter for a
        // non-flowchart type (sub-parsers see the blanked source).
        let e = parse_document("---\ntitle: X\nconfig: {}\n---\nsequenceDiagram\nnote over A:")
            .unwrap_err();
        assert_eq!(e.line, 6, "seq body error line must survive blanking");

        // Both entry points agree on a frontmatter'd flowchart.
        let a = parse("---\nt: x\n---\nA --> B").unwrap();
        let b = match parse_document("---\nt: x\n---\nA --> B").unwrap() {
            Document::Flowchart(g) => g,
            _ => panic!(),
        };
        assert_eq!(a.nodes.len(), b.nodes.len());
        assert_eq!(a.edges.len(), b.edges.len());
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

    #[test]
    fn gitgraph_duplicate_commit_id_errors() {
        let e = parse_document("gitGraph\ncommit id: \"a\"\ncommit id: \"a\"").unwrap_err();
        assert!(
            e.message.contains("duplicate commit id"),
            "got: {}",
            e.message
        );
    }

    #[test]
    fn gitgraph_default_id_skips_claimed_numbers() {
        let d = match parse_document("gitGraph\ncommit id: \"2\"\ncommit").unwrap() {
            Document::GitGraph(g) => g,
            other => panic!("expected gitGraph, got {:?}", other),
        };
        assert_eq!(d.commits[0].id, "2");
        assert_eq!(d.commits[1].id, "3");
    }

    #[test]
    fn gitgraph_unknown_command_errors() {
        let e = parse_document("gitGraph\ncherry-pick foo").unwrap_err();
        assert!(
            e.message.contains("unknown gitGraph command"),
            "got: {}",
            e.message
        );
    }

    #[test]
    fn gitgraph_duplicate_branch_errors() {
        let e = parse_document("gitGraph\nbranch feat\nbranch feat").unwrap_err();
        assert!(e.message.contains("already exists"), "got: {}", e.message);
    }

    #[test]
    fn gitgraph_merge_missing_branch_errors() {
        let e = parse_document("gitGraph\ncommit\nmerge nosuch").unwrap_err();
        assert!(e.message.contains("does not exist"), "got: {}", e.message);
    }

    #[test]
    fn gitgraph_merge_self_errors() {
        let e = parse_document("gitGraph\ncommit\nmerge main").unwrap_err();
        assert!(e.message.contains("into itself"), "got: {}", e.message);
    }

    fn gitgraph(src: &str) -> GitGraph {
        match parse_document(src).unwrap_or_else(|e| panic!("{e}\n--\n{src}")) {
            Document::GitGraph(g) => g,
            other => panic!("expected gitGraph, got {:?}", other),
        }
    }

    /// Report P1: mermaid's documented `gitGraph TB:` (and `LR:`, `BT:`,
    /// a bare `gitGraph:`) was "unknown gitGraph orientation: 'TB:'".
    #[test]
    fn gitgraph_header_takes_mermaids_colon_forms() {
        for (header, want) in [
            ("gitGraph TB:", GitOrientation::TB),
            ("gitGraph LR:", GitOrientation::LR),
            ("gitGraph BT:", GitOrientation::BT),
            ("gitGraph BT :", GitOrientation::BT),
            ("gitGraph:", GitOrientation::LR),
            ("gitGraph TB", GitOrientation::TB),
            ("gitGraph", GitOrientation::LR),
        ] {
            let g = gitgraph(&format!("{header}\ncommit"));
            assert_eq!(g.orientation, want, "{header}");
        }
        let e = parse_document("gitGraph XX:\ncommit").unwrap_err();
        assert!(
            e.message.contains("unknown gitGraph orientation"),
            "{}",
            e.message
        );
    }

    /// Report P1: `branch dev order: 2` made a branch literally named
    /// "dev order: 2". The option now orders the lanes as in mermaid:
    /// by `order`, an unordered branch after main and before order 1.
    #[test]
    fn gitgraph_branch_order_is_an_option_not_part_of_the_name() {
        let g = gitgraph(
            "gitGraph\ncommit\nbranch dev order: 3\nbranch feat order: 1\n\
             branch spike\nbranch \"release x\" order:2\nbranch my feature",
        );
        let names: Vec<&str> = g.branches.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            names,
            ["main", "dev", "feat", "spike", "release x", "my feature"]
        );
        let lane = |n: &str| g.branches.iter().find(|b| b.name == n).unwrap().order;
        // main, then the unordered (creation order), then by order.
        assert_eq!(
            [
                lane("main"),
                lane("spike"),
                lane("my feature"),
                lane("feat"),
                lane("release x"),
                lane("dev")
            ],
            [0, 1, 2, 3, 4, 5]
        );
        // `checkout` / `merge` find the branch by its bare name.
        let g = gitgraph("gitGraph\ncommit\nbranch dev order: 1\ncommit\ncheckout main\nmerge dev");
        assert_eq!(g.commits[2].second_parent, Some(1));
        let e = parse_document("gitGraph\nbranch dev order: soon").unwrap_err();
        assert!(e.message.contains("branch order"), "{}", e.message);
    }

    /// Report P1: `merge x type: HIGHLIGHT` parsed but the type was
    /// dropped (merges were hard-coded NORMAL).
    #[test]
    fn gitgraph_merge_keeps_its_type() {
        let g = gitgraph(
            "gitGraph\ncommit\nbranch a\ncommit\nbranch b\ncommit\ncheckout main\n\
             merge a type: HIGHLIGHT\nmerge b id: \"m\" type: REVERSE tag: \"v1\"",
        );
        assert_eq!(g.commits[3].kind, CommitKind::Highlight);
        assert_eq!(g.commits[4].kind, CommitKind::Reverse);
        assert_eq!(g.commits[4].id, "m");
        assert_eq!(g.commits[4].tag.as_deref(), Some("v1"));
        let e =
            parse_document("gitGraph\ncommit\nbranch a\ncommit\ncheckout main\nmerge a type: BOLD")
                .unwrap_err();
        assert!(e.message.contains("unknown commit type"), "{}", e.message);
    }

    /// Mermaid takes quoted branch references and commit messages
    /// (`msg: "…"` or a bare string); both used to be parse errors.
    #[test]
    fn gitgraph_quoted_names_and_messages() {
        let g = gitgraph(
            "gitGraph\ncommit msg: \"first\" id: \"a\"\ncommit \"second\"\n\
             branch \"my topic\"\ncheckout \"my topic\"\ncommit\nswitch \"main\"\n\
             merge \"my topic\" id: \"m\"",
        );
        assert_eq!(g.branches[1].name, "my topic");
        assert_eq!(g.commits[0].id, "a");
        assert_eq!(g.commits[3].id, "m");
        assert_eq!(g.commits[3].second_parent, Some(2));
    }

    #[test]
    fn gitgraph_unknown_attribute_errors() {
        let e = parse_document("gitGraph\ncommit foo: \"bar\"").unwrap_err();
        assert!(
            e.message.contains("unknown gitGraph attribute"),
            "got: {}",
            e.message
        );
    }

    // ── flowchart-elk header + accTitle / accDescr ─────────────────

    fn svg_of(src: &str) -> String {
        crate::render_svg(src).unwrap_or_else(|e| panic!("{e}\n--\n{src}"))
    }

    #[test]
    fn flowchart_elk_is_a_flowchart_alias() {
        for (elk, plain) in [
            ("flowchart-elk\n  A --> B\n", "flowchart\n  A --> B\n"),
            ("flowchart-elk TD\n  A --> B\n", "flowchart TD\n  A --> B\n"),
            (
                "flowchart-elk LR\n  A --> B --> C\n",
                "flowchart LR\n  A --> B --> C\n",
            ),
            ("flowchart-elk RL\n  A --> B\n", "flowchart RL\n  A --> B\n"),
            ("flowchart-elk BT\n  A --> B\n", "flowchart BT\n  A --> B\n"),
        ] {
            assert_eq!(svg_of(elk), svg_of(plain), "{elk}");
            assert!(matches!(
                parse_document(elk).unwrap(),
                Document::Flowchart(_)
            ));
        }
        assert_eq!(parse("flowchart-elk").unwrap().direction, Direction::TD);
        assert_eq!(parse("flowchart-elk LR").unwrap().direction, Direction::LR);
        // Same leniency as `flowchart` (strip_keyword is case-insensitive).
        assert_eq!(
            parse("FLOWCHART-ELK LR\nA").unwrap().direction,
            Direction::LR
        );
        let e = parse("flowchart-elk XY\nA").unwrap_err();
        assert!(e.message.contains("unknown direction"), "{}", e.message);
    }

    #[test]
    fn flowchart_elk_is_not_a_header_inside_the_body_or_without_whitespace() {
        // Second header after the first: an error, as in mermaid.
        assert!(parse("flowchart TD\n  A --> B\n  flowchart-elk LR\n").is_err());
        // Glued direction is not the header (same rule as `flowchartTD`).
        assert!(parse("flowchart-elkTD\nA --> B").is_err());
        // A longer word is never the header keyword.
        let baseline = parse("flowchart-elkish --> B").map(|g| g.nodes.len());
        assert!(baseline.is_err(), "flowmaid node ids are word-like");
    }

    fn acc(src: &str) -> (Option<String>, Option<String>) {
        let g = parse(src).unwrap_or_else(|e| panic!("{e}\n--\n{src}"));
        (g.acc_title, g.acc_descr)
    }

    #[test]
    fn acc_title_and_descr_single_line() {
        let src = "flowchart LR\n  accTitle: Order pipeline\n  accDescr: Orders flow from cart to payment\n  A --> B\n";
        assert_eq!(
            acc(src),
            (
                Some("Order pipeline".into()),
                Some("Orders flow from cart to payment".into())
            )
        );
        let g = parse(src).unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        let svg = svg_of(src);
        assert!(svg.contains("aria-label=\"Order pipeline\""), "{svg}");
        assert!(svg.contains("<title>Order pipeline</title>\n<desc id=\"flowmaid-desc-"));
        let id = svg
            .split("aria-describedby=\"")
            .nth(1)
            .and_then(|r| r.split('"').next())
            .unwrap();
        assert!(id.starts_with(A11Y_DESC_ID_PREFIX_TEST), "{id}");
        assert!(svg.contains(&format!(
            "<desc id=\"{id}\">Orders flow from cart to payment</desc>"
        )));
        assert!(!svg.contains("Flowchart diagram"));
    }

    const A11Y_DESC_ID_PREFIX_TEST: &str = "flowmaid-desc-";

    #[test]
    fn acc_value_spacing_and_empty_values() {
        assert_eq!(acc("flowchart TD\naccTitle:x\nA").0.as_deref(), Some("x"));
        assert_eq!(
            acc("flowchart TD\naccTitle :  x  \nA").0.as_deref(),
            Some("x")
        );
        // Empty title: no title, output identical to no statement.
        let src = "flowchart TD\n  accTitle:\n  A-->B\n";
        assert_eq!(acc(src), (None, None));
        assert_eq!(svg_of(src), svg_of("flowchart TD\n  A-->B\n"));
        assert_eq!(acc("flowchart TD\n  accDescr:   \n  A-->B\n"), (None, None));
    }

    #[test]
    fn acc_descr_block_normalises_like_mermaid() {
        let src = "flowchart TD\n  accDescr {\n    First line\n      indented second\n\n    after blank\n  }\n  A --> B\n";
        let (t, d) = acc(src);
        assert_eq!(t, None);
        assert_eq!(
            d.as_deref(),
            Some("First line\nindented second\nafter blank")
        );
        let svg = svg_of(src);
        assert!(svg.contains("<title>Flowchart diagram</title>"));
        assert!(svg.contains(">First line\nindented second\nafter blank</desc>"));
        assert_eq!(parse(src).unwrap().edges.len(), 1);
    }

    #[test]
    fn acc_descr_block_one_line_and_glued_and_trailing_statements() {
        assert_eq!(
            acc("flowchart TD\n  accDescr { short }\n  A --> B\n")
                .1
                .as_deref(),
            Some("short")
        );
        let src = "flowchart TD\n  accDescr{\nx\n} A --> B\n";
        assert_eq!(acc(src).1.as_deref(), Some("x"));
        let g = parse(src).unwrap();
        assert_eq!(g.edges.len(), 1, "edge after '}}' still parses");
        assert_eq!(g.nodes[0].id, "A");
        // `accDescr{label}` is a description, not a rhombus node.
        let g = parse("flowchart TD\naccDescr{label}\nA").unwrap();
        assert_eq!(g.acc_descr.as_deref(), Some("label"));
        assert_eq!(g.nodes.len(), 1);
        // Content on the `{` line; `%%` lines inside are dropped.
        assert_eq!(
            acc("flowchart TD\naccDescr { a\n%% hidden\n  b }\nA")
                .1
                .as_deref(),
            Some("a\nb")
        );
        // A single-line value keeps a `}`.
        assert_eq!(
            acc("flowchart TD\naccDescr: a } b\nA").1.as_deref(),
            Some("a } b")
        );
    }

    #[test]
    fn acc_title_runs_to_end_of_line_through_semicolons() {
        let src = "flowchart TD\n  accTitle: a; B --> C\n  A --> D\n";
        let g = parse(src).unwrap();
        assert_eq!(g.acc_title.as_deref(), Some("a; B --> C"));
        assert!(g.node_index("B").is_none() && g.node_index("C").is_none());
        assert_eq!(g.edges.len(), 1);
        // After a `;` a new statement starts.
        let g = parse("flowchart TD\n  A --> B; accTitle: T2\n").unwrap();
        assert_eq!(g.acc_title.as_deref(), Some("T2"));
        assert_eq!(g.edges.len(), 1);
        // `%%` stays in a jison value.
        assert_eq!(
            acc("flowchart TD\naccTitle: x %% note\nA").0.as_deref(),
            Some("x %% note")
        );
    }

    #[test]
    fn acc_last_statement_wins() {
        let src = "flowchart TD\n  accTitle: one\n  accTitle: two\n  accDescr: d1\n  accDescr {\n d2\n }\n  A-->B\n";
        assert_eq!(acc(src), (Some("two".into()), Some("d2".into())));
        let src = "flowchart TD\naccDescr {\nd1\n}\naccDescr: d2\nA";
        assert_eq!(acc(src).1.as_deref(), Some("d2"));
    }

    #[test]
    fn acc_markup_is_escaped_plain_text() {
        let src = "flowchart TD\n  accTitle: <b>Bold</b> & \"q\" #quot;\n  accDescr: <i>x</i> & y\n  A-->B\n";
        let svg = svg_of(src);
        assert!(
            svg.contains("<title>Bold &amp; &quot;q&quot; #quot;</title>"),
            "{svg}"
        );
        assert!(svg.contains("aria-label=\"Bold &amp; &quot;q&quot; #quot;\""));
        assert!(
            svg.contains(">&lt;i&gt;x&lt;/i&gt; &amp; y</desc>"),
            "{svg}"
        );
        assert!(!svg.contains("<b>"));
        // Control characters never reach the SVG.
        let svg = svg_of("flowchart TD\naccDescr: a\u{1}b\nA");
        assert!(svg.contains(">ab</desc>"));
    }

    #[test]
    fn acc_inside_subgraph_sets_the_diagram_value() {
        let src = "flowchart TD\n  subgraph S\n    accTitle: Inside\n    A --> B\n  end\n";
        let g = parse(src).unwrap();
        assert_eq!(g.acc_title.as_deref(), Some("Inside"));
        assert_eq!(g.subgraphs.len(), 1);
        let ids: Vec<_> = g.subgraphs[0]
            .nodes
            .iter()
            .map(|&n| g.nodes[n].id.as_str())
            .collect();
        assert_eq!(ids, ["A", "B"]);
    }

    #[test]
    fn acc_keywords_are_case_sensitive_and_need_their_punctuation() {
        let g = parse("flowchart TD\n  accTitle --> B\n").unwrap();
        assert_eq!(g.nodes[0].id, "accTitle");
        assert_eq!(g.acc_title, None);
        let g = parse("flowchart TD\n  accDescr --> B\n").unwrap();
        assert_eq!(g.nodes[0].id, "accDescr");
        let base = "flowchart TD\n  AccTitle: x\n";
        let e = parse(base).unwrap_err();
        assert!(e.message.contains("unknown edge operator"), "{}", e.message);
        assert!(parse("flowchart TD\n  ACCDESCR: x\n").is_err());
    }

    #[test]
    fn acc_unclosed_block_is_a_line_numbered_error() {
        let e = parse("flowchart TD\n  accDescr {\n  never closed\n  A --> B\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("never closed"), "{}", e.message);
    }

    #[test]
    fn acc_line_numbers_after_a_block_stay_exact() {
        let e = parse("flowchart TD\naccDescr {\na\nb\n}\nA --> ???\n").unwrap_err();
        assert_eq!(e.line, 6);
    }

    #[test]
    fn acc_before_the_header_is_not_acc() {
        assert!(parse("accTitle: x\nflowchart TD\nA").is_err());
    }

    #[test]
    fn state_diagram_acc() {
        let src = "stateDiagram-v2\n  accTitle: Door states\n  accDescr: Open and closed\n  [*] --> Open\n";
        let Document::State(g) = parse_document(src).unwrap() else {
            panic!("state")
        };
        assert_eq!(g.acc_title.as_deref(), Some("Door states"));
        assert_eq!(g.acc_descr.as_deref(), Some("Open and closed"));
        assert!(g
            .nodes
            .iter()
            .all(|n| !n.label.contains("Door") && !n.label.contains("Open and")));
        let svg = svg_of(src);
        assert!(svg.contains("aria-label=\"Door states\""));
        assert!(svg.contains(">Open and closed</desc>"));
        assert!(!svg.contains("State diagram"));
        // Without acc: unchanged default name.
        let svg = svg_of("stateDiagram-v2\n  [*] --> Open\n");
        assert!(svg.contains("<title>State diagram</title>"));
        assert!(!svg.contains("<desc"));
        // A multi-line block inside a composite state.
        let src = "stateDiagram-v2\nstate G {\naccDescr {\n x\n}\nA --> B\n}\n";
        let Document::State(g) = parse_document(src).unwrap() else {
            panic!("state")
        };
        assert_eq!(g.acc_descr.as_deref(), Some("x"));
    }

    #[test]
    fn class_sequence_er_acc() {
        let svg = svg_of("classDiagram\n  accTitle: T1\n  accDescr: D1\n  class A\n");
        assert!(svg.contains("<title>T1</title>\n<desc id=\""), "{svg}");
        assert!(!svg.contains(">accTitle<") && !svg.contains(">accDescr<"));
        let Document::Class(d) =
            parse_document("classDiagram\n  accTitle: T1\n  accDescr: D1\n  class A\n").unwrap()
        else {
            panic!()
        };
        assert_eq!(d.classes.len(), 1);
        let svg = svg_of("sequenceDiagram\n  accTitle: S\n  accDescr {\n a\n b\n }\n  A->>B: hi\n");
        assert!(svg.contains("<title>S</title>\n<desc id=\""), "{svg}");
        assert!(svg.contains(">a\nb</desc>"));
        let svg = svg_of("erDiagram\n  accTitle: E\n  A ||--o{ B : has\n");
        assert!(svg.contains("aria-label=\"E\""));
        // Defaults unchanged without acc.
        assert!(svg_of("classDiagram\n  class A\n").contains("<title>Class diagram</title>"));
    }

    #[test]
    fn other_types_accept_acc_without_drawing_it() {
        let Document::Timeline(t) =
            parse_document("timeline\n  accTitle: T1\n  2020 : x\n").unwrap()
        else {
            panic!()
        };
        let periods: Vec<_> = t
            .sections
            .iter()
            .flat_map(|s| &s.periods)
            .map(|p| p.period.clone())
            .collect();
        assert_eq!(periods, ["2020"]);
        // langium: `%%` ends the value, and acc no longer errors.
        assert!(parse_document("pie\n  accTitle: T1 %% c\n  \"a\": 1\n").is_ok());
        assert!(parse_document("journey\n  accTitle: J\n  section S\n  t: 5: me\n").is_ok());
        assert!(parse_document("gitGraph\n  accDescr {\n x\n }\n  commit\n").is_ok());
        assert!(
            parse_document("architecture-beta\n  accTitle: A\n  service db(database)[DB]\n")
                .is_ok()
        );
        // Mermaid has no acc in mindmap: unchanged (a node named so).
        let Document::Mindmap(m) = parse_document("mindmap\n  root\n    accTitle: x\n").unwrap()
        else {
            panic!()
        };
        assert!(format!("{m:?}").contains("accTitle"));
    }

    #[test]
    fn acc_langium_values() {
        let (src, a) = extract_acc(
            "pie\naccTitle:  a   b  %% c\naccDescr {\n  x   y\n\n    z\n}\n",
            1,
            false,
            AccFlavor::Langium,
        )
        .unwrap();
        assert_eq!(a.title.as_deref(), Some("a b"));
        assert_eq!(a.descr.as_deref(), Some("x y\nz"));
        assert!(src.lines().skip(1).all(|l| l.trim().is_empty()));
    }

    // ── edge ids (`A e1@--> B`) + edge metadata (`e1@{ … }`) ───────

    fn edge_ids(g: &Graph) -> Vec<Option<&str>> {
        g.edges.iter().map(|e| e.meta.id.as_deref()).collect()
    }

    #[test]
    fn edge_id_on_every_operator_form() {
        let g = parse("flowchart LR\n  A e1@--> B\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e1")]);
        assert_eq!(g.nodes.len(), 2);
        let g = parse("flowchart LR\n  A e1@ --> B\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e1")]);
        let g = parse("flowchart LR\n  A e1@-- go --> B\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e1")]);
        assert_eq!(g.edges[0].label.as_deref(), Some("go"));
        let g = parse("flowchart LR\n  A e1@-->|go| B\n").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("go"));
        assert_eq!(edge_ids(&g), [Some("e1")]);
        let g = parse("flowchart LR\n  A e1@x--x B\n  C e2@<--> D\n  E e3@--o F\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e1"), Some("e2"), Some("e3")]);
        assert_eq!(g.edges[0].ends.start, Some(Marker::Cross));
        assert_eq!(g.edges[1].ends.start, Some(Marker::Arrow));
        assert_eq!(g.edges[2].ends.end, Some(Marker::Circle));
        let g = parse("flowchart LR\n  A e1@-.-> B\n  C e2@==> D\n  E e3@----> F\n  G e4@~~~ H\n")
            .unwrap();
        let kinds: Vec<_> = g.edges.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            [
                EdgeKind::Dotted,
                EdgeKind::Thick,
                EdgeKind::Arrow,
                EdgeKind::Invisible
            ]
        );
        assert_eq!(
            edge_ids(&g),
            [Some("e1"), Some("e2"), Some("e3"), Some("e4")]
        );
        // Mermaid's id charset (`-`, `.`, digits) and its first-`@` strip.
        let g = parse("flowchart LR\n  A e-1@--> B\n  B 1@--> C\n  C a@b@--> D\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e-1"), Some("1"), Some("ab@")]);
    }

    #[test]
    fn edge_id_fan_out_chain_and_duplicates() {
        let g = parse("flowchart LR\n  A & B e1@--> C & D\n").unwrap();
        let named: Vec<_> = g
            .edges
            .iter()
            .map(|e| {
                (
                    g.nodes[e.from].id.as_str(),
                    g.nodes[e.to].id.as_str(),
                    e.meta.id.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            named,
            [
                ("A", "C", None),
                ("A", "D", None),
                ("B", "C", Some("e1")),
                ("B", "D", None)
            ]
        );
        let g = parse("flowchart LR\n  A e1@--> B e2@--> C\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e1"), Some("e2")]);
        let g = parse("flowchart LR\n  A e1@--> B\n  C e1@--> D\n").unwrap();
        assert_eq!(edge_ids(&g), [Some("e1"), None]);
        // Glued to the source node: mermaid's lexer error, ours too.
        assert!(parse("flowchart LR\n  Ae1@-->B\n").is_err());
        // `@` not before an operator is not an edge id.
        assert!(parse("flowchart LR\n  A e1@ B\n").is_err());
    }

    fn meta_of(src: &str) -> EdgeMeta {
        parse(src)
            .unwrap_or_else(|e| panic!("{e}\n--\n{src}"))
            .edges[0]
            .meta
            .clone()
    }

    #[test]
    fn edge_metadata_animation_follows_mermaids_rule() {
        let base = "flowchart LR\n  A e1@--> B\n";
        let anim = |m: &str| meta_of(&format!("{base}  e1@{{ {m} }}\n")).animation;
        assert_eq!(anim("animate: true"), Some(EdgeAnimation::Fast));
        assert_eq!(anim("animation: slow"), Some(EdgeAnimation::Slow));
        assert_eq!(anim("animation: fast"), Some(EdgeAnimation::Fast));
        assert_eq!(
            anim("animate: false, animation: fast"),
            Some(EdgeAnimation::Fast)
        );
        assert_eq!(anim("animate: true, animation: bogus"), None);
        assert_eq!(anim("animation: bogus"), None);
        assert_eq!(anim("animate: yes"), Some(EdgeAnimation::Fast));
        assert_eq!(anim("animate: \"false\""), Some(EdgeAnimation::Fast));
        assert_eq!(anim("animate: 0"), None);
        assert_eq!(anim("animate: null"), None);
        assert_eq!(
            anim("animate: true, animation: \"\""),
            Some(EdgeAnimation::Fast)
        );
        // Unknown / node keys on an edge: ignored, no node minted.
        let g = parse(&format!("{base}  e1@{{ shape: rect, foo: 1 }}\n")).unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges[0].meta.animation, None);
    }

    #[test]
    fn edge_metadata_merges_and_spans_lines() {
        let m =
            meta_of("flowchart LR\n  A e1@--> B\n  e1@{ animate: true }\n  e1@{ curve: linear }\n");
        assert_eq!(m.animation, Some(EdgeAnimation::Fast));
        assert_eq!(m.curve, Some(EdgeCurve::Linear));
        let m = meta_of(
            "flowchart LR\n  A e1@--> B\n  e1@{ animation: slow }\n  e1@{ animation: fast }\n",
        );
        assert_eq!(m.animation, Some(EdgeAnimation::Fast));
        let src = "flowchart LR\n  A e1@--> B\n  e1@{\n    animate: true\n    %% a comment line\n    curve: step\n  }\n  B --> C\n";
        let g = parse(src).unwrap();
        assert_eq!(g.edges[0].meta.curve, Some(EdgeCurve::Step));
        assert_eq!(g.edges[0].meta.animation, Some(EdgeAnimation::Fast));
        assert_eq!(g.edges.len(), 2);
        // Block form keeps a trailing comma in the value (mermaid).
        let m = meta_of("flowchart LR\n  A e1@--> B\n  e1@{\n    curve: linear,\n  }\n");
        assert_eq!(m.curve, Some(EdgeCurve::Basis));
        // Line numbers after a joined block stay exact.
        let e =
            parse("flowchart LR\nA e1@--> B\ne1@{\n animate: true\n}\nA --> ???\n").unwrap_err();
        assert_eq!(e.line, 6);
    }

    #[test]
    fn edge_metadata_curves() {
        let src = "flowchart LR\n  A e1@==> B\n  A e2@--> C\n  e1@{ curve: linear }\n  e2@{ curve: natural }\n";
        let g = parse(src).unwrap();
        assert_eq!(g.edges[0].meta.curve, Some(EdgeCurve::Linear));
        assert_eq!(g.edges[1].meta.curve, Some(EdgeCurve::Natural));
        for (name, c) in [
            ("basis", EdgeCurve::Basis),
            ("stepBefore", EdgeCurve::StepBefore),
            ("stepAfter", EdgeCurve::StepAfter),
            ("rounded", EdgeCurve::Rounded),
            ("bumpX", EdgeCurve::BumpX),
            ("bumpY", EdgeCurve::BumpY),
            ("cardinal", EdgeCurve::Cardinal),
            ("catmullRom", EdgeCurve::CatmullRom),
            ("monotoneX", EdgeCurve::MonotoneX),
            ("monotoneY", EdgeCurve::MonotoneY),
            ("wiggly", EdgeCurve::Basis),
            ("Linear", EdgeCurve::Basis),
        ] {
            let m = meta_of(&format!(
                "flowchart TD\n  A e1@--> B\n  e1@{{ curve: {name} }}\n"
            ));
            assert_eq!(m.curve, Some(c), "{name}");
        }
        assert_eq!(
            meta_of("flowchart TD\nA e1@--> B\ne1@{ curve: 0 }").curve,
            None
        );
    }

    #[test]
    fn edge_metadata_errors_and_routing() {
        let e = parse("flowchart LR\n  A e1@--> B\n  e1@{ animate: true\n").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("never closed"), "{}", e.message);
        let e = parse("flowchart LR\n  A e1@--> B\n  e1@{ animate: true curve: x }\n").unwrap_err();
        assert!(e.message.contains("missed comma"), "{}", e.message);
        assert!(parse("flowchart LR\n  A e1@--> B\n  e1@{ animate: true } --> C\n").is_err());
        // A sub-edge carries its id + metadata.
        let src =
            "flowchart LR\n  subgraph S\n    X\n  end\n  A e1@--> S\n  e1@{ animation: fast }\n";
        let g = parse(src).unwrap();
        assert_eq!(g.sub_edges[0].meta.id.as_deref(), Some("e1"));
        assert_eq!(g.sub_edges[0].meta.animation, Some(EdgeAnimation::Fast));
        // Subgraph metadata is accepted and dropped.
        let g =
            parse("flowchart TB\n  subgraph s1\n    a\n  end\n  s1@{ view: collapsed }\n").unwrap();
        assert_eq!(g.nodes.len(), 1);
        // `class` / `style` on an edge id never mint a stray node.
        let src = "flowchart LR\n  A e1@--> B\n  classDef animate stroke-dashoffset: 900,animation: dash 25s linear infinite\n  class e1 animate\n  style e1 stroke:#f00\n";
        let g = parse(src).unwrap();
        assert!(g.node_index("e1").is_none(), "{:?}", g.nodes);
        // `@` inside a quoted label is text, not metadata or an id.
        let g = parse("flowchart TB\n  X[\"a @{ b }\"] --> Y\n  M[\"user@host\"] --> N\n").unwrap();
        assert_eq!(g.nodes[0].label, "a @{ b }");
        assert_eq!(edge_ids(&g), [None, None]);
    }

    // ── node metadata `id@{ … }` ───────────────────────────────────

    fn node<'g>(g: &'g Graph, id: &str) -> &'g crate::model::Node {
        &g.nodes[g
            .node_index(id)
            .unwrap_or_else(|| panic!("no node {id}: {:?}", g.nodes))]
    }

    fn meta_err(src: &str) -> String {
        parse(src)
            .map(|g| format!("{:?}", g.nodes))
            .expect_err(src)
            .message
    }

    #[test]
    fn node_meta_shape_and_label() {
        let g = parse("flowchart TD\n  A@{ shape: cyl, label: \"DB\" } --> B\n").unwrap();
        assert_eq!(
            (node(&g, "A").shape, node(&g, "A").label.as_str()),
            (Shape::Cylinder, "DB")
        );
        assert_eq!(node(&g, "B").shape, Shape::Rect);
        assert_eq!(g.edges.len(), 1);
        let g = parse("flowchart TB\n  D@{ shape: rounded}\n  E@{shape: rounded}\n  F@{       shape: rounded         }\n").unwrap();
        for id in ["D", "E", "F"] {
            assert_eq!(node(&g, id).shape, Shape::Rounded);
            assert_eq!(node(&g, id).label, id);
        }
        let g = parse("flowchart TB\n  D@{ shape: rounded , label: \"DD\"}\n").unwrap();
        assert_eq!(node(&g, "D").label, "DD");
        let g = parse("flowchart TB\n  D@{ shape: rounded } & E@{ shape: rounded } --> F@{ shape: rounded } & G@{ shape: rounded }\n").unwrap();
        assert_eq!((g.nodes.len(), g.edges.len()), (4, 4));
        assert!(g.nodes.iter().all(|n| n.shape == Shape::Rounded));
        let g = parse("flowchart TB\n  A --> B@{ shape: hex } --> C\n").unwrap();
        assert_eq!(node(&g, "B").shape, Shape::Hexagon);
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn node_meta_multi_line_forms() {
        let g = parse("flowchart TB\n  A --> D@{\n    shape: circle\n    other: \"clock\"\n  }\n")
            .unwrap();
        assert_eq!(node(&g, "D").shape, Shape::Circle);
        assert_eq!(g.edges.len(), 1);
        let g = parse("flowchart TB\n  A[hello]\n  B@{\n    shape: circle\n  }\n  C[Hello]@{\n    shape: circle\n  }\n").unwrap();
        assert_eq!(
            (node(&g, "C").shape, node(&g, "C").label.as_str()),
            (Shape::Circle, "Hello")
        );
        assert_eq!(node(&g, "B").shape, Shape::Circle);
        let g = parse("flowchart TB\n  A@{\n    label: \"This is }\"\n  }\n").unwrap();
        assert_eq!(node(&g, "A").label, "This is }");
        let g =
            parse("flowchart TB\n  A@{\n    label: \"This is a\n      multiline string\"\n  }\n")
                .unwrap();
        assert_eq!(node(&g, "A").label, "This is a\nmultiline string");
        let g = parse(
            "flowchart TB\n  A@{\n    label: |\n      This is a\n      multiline string\n  }\n",
        )
        .unwrap();
        assert_eq!(
            node(&g, "A").label,
            "This is a\nmultiline string",
            "trailing newline trimmed"
        );
        let g = parse("flowchart TB\n  A@{\n    %% comment\n    shape: cyl\n  }\n").unwrap();
        assert_eq!(node(&g, "A").shape, Shape::Cylinder);
        let e = parse("flowchart TB\n  A@{ shape: cyl\n").unwrap_err();
        assert!(e.message.contains("never closed"), "{}", e.message);
    }

    #[test]
    fn node_meta_mermaid_errors() {
        assert_eq!(
            meta_err("flowchart TB\n  A@{ shape: this-shape-does-not-exist }\n"),
            "No such shape: this-shape-does-not-exist."
        );
        assert_eq!(
            meta_err("flowchart TB\n  A@{ shape: rect_left_inv_arrow }\n"),
            "No such shape: rect_left_inv_arrow. Shape names should be lowercase."
        );
        assert!(meta_err("flowchart TB\n  A@{ shape: Rect }\n").contains("lowercase"));
        assert_eq!(
            meta_err("flowchart TB\n  A@{ shape: square }\n"),
            "No such shape: square."
        );
        assert!(meta_err("flowchart TB\n  A@{ shape: cyl label: x }\n").contains("missed comma"));
        assert!(meta_err("flowchart TB\n  A@{ shape: cyl, shape: rect }\n").contains("duplicated"));
        assert!(meta_err("flowchart TB\n  A@{ shape: 1 }\n").contains("shape"));
        assert!(
            parse("flowchart TB\n  A @{ shape: cyl }\n").is_err(),
            "space before @"
        );
        assert!(
            parse("flowchart TB\n  A@ { shape: cyl }\n").is_err(),
            "space after @"
        );
        assert!(parse("flowchart TB\n  A@{ shape: hex }:::hot\n").is_err());
        // Single quotes don't protect `}` in mermaid's lexer.
        assert!(parse("flowchart TB\n  A@{ label: 'a}b' }\n").is_err());
        // A `;` inside the braces is YAML text, not a statement split.
        assert!(meta_err("flowchart TB\n  A@{ shape: cyl; label: x }\n").contains("missed comma"));
    }

    #[test]
    fn node_meta_values_follow_js_yaml_and_truthiness() {
        let g = parse("flowchart TB\n  A@{ label: Hello, world }\n").unwrap();
        assert_eq!(node(&g, "A").label, "Hello");
        let g = parse("flowchart TB\n  A@{ shape:cyl }\n").unwrap();
        assert_eq!(
            (node(&g, "A").shape, node(&g, "A").label.as_str()),
            (Shape::Rect, "A")
        );
        let g = parse("flowchart TB\n  A@{ label: \"\" }\n  B@{}\n").unwrap();
        assert_eq!(
            (node(&g, "A").label.as_str(), node(&g, "B").label.as_str()),
            ("A", "B")
        );
        let g = parse("flowchart TB\n  A@{ label: 123 }\n  B@{ shape: \"\" }\n").unwrap();
        assert_eq!(node(&g, "A").label, "123");
        assert_eq!(node(&g, "B").shape, Shape::Rect);
        let g = parse("flowchart TB\n  A@{ shape: doublecircle }\n").unwrap();
        assert_eq!(node(&g, "A").shape, Shape::DoubleCircle);
        let g = parse("flowchart TB\n  A@{ label: \"#quot;q#quot; a<br/>b\" }\n").unwrap();
        assert_eq!(node(&g, "A").label, "\"q\" a\nb");
    }

    #[test]
    fn node_meta_labels_are_markdown_unless_typed_plain() {
        // Mermaid 12: no `labelType` (or an unknown one) = markdown.
        for src in [
            "flowchart TB\n  A@{ label: \"**b** _i_\" }\n",
            "flowchart TB\n  A@{ label: \"**b** _i_\", labelType: markdown }\n",
            "flowchart TB\n  A@{ label: \"**b** _i_\", labelType: bogus }\n",
        ] {
            assert_eq!(
                node(&parse(src).unwrap(), "A").label,
                "<b>b</b> <i>i</i>",
                "{src}"
            );
        }
        // Same lowering as the ``"`…`"`` bracket form, wrapping included.
        let long = "one two three four five six seven eight nine ten eleven twelve";
        assert_eq!(
            node(
                &parse(&format!("flowchart TB\n  A@{{ label: \"{long}\" }}\n")).unwrap(),
                "A"
            )
            .label,
            node(
                &parse(&format!("flowchart TB\n  A[\"`{long}`\"]\n")).unwrap(),
                "A"
            )
            .label,
        );
        // `string` / `text` keep it plain.
        for t in ["string", "text"] {
            let src = format!("flowchart TB\n  A@{{ label: \"**b** _i_\", labelType: {t} }}\n");
            assert_eq!(node(&parse(&src).unwrap(), "A").label, "**b** _i_", "{src}");
        }
        // A backtick inside a YAML label is a plain character (it never
        // went through the markdown-string lexer).
        let g = parse("flowchart TB\n  A@{ label: \"a `x` b\" }\n").unwrap();
        assert_eq!(node(&g, "A").label, "a `x` b");
    }

    #[test]
    fn node_meta_order_class_and_subgraphs() {
        let g =
            parse("flowchart TB\n  A:::hot@{ shape: hex }\n  classDef hot fill:#f00\n").unwrap();
        assert_eq!(node(&g, "A").shape, Shape::Hexagon);
        assert_eq!(node(&g, "A").style.fill.as_deref(), Some("#f00"));
        let g = parse("flowchart TB\n  A --> B\n  B@{ shape: cyl }\n  B[Again]\n").unwrap();
        assert_eq!(
            (node(&g, "B").shape, node(&g, "B").label.as_str()),
            (Shape::Rect, "Again")
        );
        let g = parse("flowchart TB\n  A@{ label: \"L\" }\n  A\n").unwrap();
        assert_eq!(node(&g, "A").label, "L", "a bare mention changes nothing");
        let g = parse("flowchart TB\n  A@{ shape: cyl }; B@{ shape: doc } --> A\n").unwrap();
        assert_eq!(node(&g, "A").shape, Shape::Cylinder);
        assert_eq!(g.edges.len(), 1);
        let g = parse("flowchart TB\n  subgraph s1\n    a\n  end\n  s1@{ view: collapsed }\n  x --> s1@{ view: collapsed }\n").unwrap();
        assert!(g.node_index("s1").is_none());
        assert_eq!(g.sub_edges.len(), 1);
        // Quoted `@{` is label text: unchanged from before.
        let g = parse("flowchart TB\n  X[\"a @{ b }\"] --> Y\n").unwrap();
        assert_eq!(node(&g, "X").label, "a @{ b }");
    }

    #[test]
    fn node_meta_icon_and_image_keys_parse() {
        let g = parse("flowchart TB\n  A@{ icon: \"fa:user\", form: \"square\", label: \"User Icon\", pos: \"t\", h: 60 }\n  B@{ img: \"https://example.com/i.png\", w: 60, h: 60, constraint: \"on\" }\n").unwrap();
        assert_eq!(node(&g, "A").label, "User Icon");
        assert_eq!(node(&g, "B").label, "B", "kept: no image replaces it");
    }

    #[test]
    fn node_meta_borderless_text_shapes() {
        let g = parse("flowchart TB\n  A@{ shape: text, label: hi }\n  B@{ shape: brace }\n  C@{ shape: text }\n  style C fill:#ff0\n").unwrap();
        assert_eq!(node(&g, "A").style.fill.as_deref(), Some("none"));
        assert_eq!(node(&g, "A").style.stroke.as_deref(), Some("none"));
        assert_eq!(node(&g, "B").style.stroke.as_deref(), Some("none"));
        assert_eq!(
            node(&g, "C").style.fill.as_deref(),
            Some("#ff0"),
            "style wins"
        );
        assert_eq!(node(&g, "C").style.stroke.as_deref(), Some("none"));
        // A later bracket shape or a boxed shape drops the borderless look.
        let g = parse("flowchart TB\n  A@{ shape: text }\n  A[box]\n  B@{ shape: text }\n  B@{ shape: rect }\n").unwrap();
        assert_eq!(node(&g, "A").style.fill, None);
        assert_eq!(node(&g, "B").style.fill, None);
    }

    #[test]
    fn every_mermaid_shape_name_maps() {
        let exact = [
            ("rect", Shape::Rect),
            ("rounded", Shape::Rounded),
            ("stadium", Shape::Stadium),
            ("fr-rect", Shape::Subroutine),
            ("cyl", Shape::Cylinder),
            ("circle", Shape::Circle),
            ("dbl-circ", Shape::DoubleCircle),
            ("diam", Shape::Diamond),
            ("hex", Shape::Hexagon),
            ("lean-r", Shape::Parallelogram),
            ("lean-l", Shape::ParallelogramAlt),
            ("sm-circ", Shape::StateStart),
            ("fr-circ", Shape::StateEnd),
            ("fork", Shape::ForkBar),
            ("f-circ", Shape::StateStart),
        ];
        let mut src = String::from("flowchart LR\n  ");
        for (i, (name, _)) in exact.iter().enumerate() {
            if i > 0 {
                src.push_str(" --> ");
            }
            src.push_str(&format!("n{i}@{{shape: {name}}}"));
        }
        let g = parse(&src).unwrap();
        for (i, (name, want)) in exact.iter().enumerate() {
            assert_eq!(node(&g, &format!("n{i}")).shape, *want, "{name}");
        }
        // Every registry name (mermaid HEAD) parses; the ones its
        // lowercase gate rejects error.
        let all = [
            "rect",
            "proc",
            "process",
            "rectangle",
            "rounded",
            "event",
            "stadium",
            "terminal",
            "pill",
            "fr-rect",
            "subprocess",
            "subproc",
            "framed-rectangle",
            "subroutine",
            "cyl",
            "db",
            "database",
            "cylinder",
            "datastore",
            "data-store",
            "folder",
            "directory",
            "bucket",
            "console",
            "browser",
            "person",
            "circle",
            "circ",
            "bang",
            "cloud",
            "diam",
            "decision",
            "diamond",
            "question",
            "hex",
            "hexagon",
            "prepare",
            "lean-r",
            "lean-right",
            "in-out",
            "lean-l",
            "lean-left",
            "out-in",
            "trap-b",
            "priority",
            "trapezoid-bottom",
            "trapezoid",
            "trap-t",
            "manual",
            "trapezoid-top",
            "inv-trapezoid",
            "dbl-circ",
            "double-circle",
            "doublecircle",
            "text",
            "notch-rect",
            "card",
            "notched-rectangle",
            "lin-rect",
            "lined-rectangle",
            "lined-process",
            "lin-proc",
            "shaded-process",
            "sm-circ",
            "start",
            "small-circle",
            "fr-circ",
            "stop",
            "framed-circle",
            "fork",
            "join",
            "hourglass",
            "collate",
            "brace",
            "comment",
            "brace-l",
            "brace-r",
            "braces",
            "bolt",
            "com-link",
            "lightning-bolt",
            "doc",
            "document",
            "delay",
            "half-rounded-rectangle",
            "h-cyl",
            "das",
            "horizontal-cylinder",
            "lin-cyl",
            "disk",
            "lined-cylinder",
            "curv-trap",
            "curved-trapezoid",
            "display",
            "div-rect",
            "div-proc",
            "divided-rectangle",
            "divided-process",
            "tri",
            "extract",
            "triangle",
            "win-pane",
            "internal-storage",
            "window-pane",
            "f-circ",
            "junction",
            "filled-circle",
            "notch-pent",
            "loop-limit",
            "notched-pentagon",
            "flip-tri",
            "manual-file",
            "flipped-triangle",
            "sl-rect",
            "manual-input",
            "sloped-rectangle",
            "docs",
            "documents",
            "st-doc",
            "stacked-document",
            "st-rect",
            "procs",
            "processes",
            "stacked-rectangle",
            "bow-rect",
            "stored-data",
            "bow-tie-rectangle",
            "cross-circ",
            "summary",
            "crossed-circle",
            "tag-doc",
            "tagged-document",
            "tag-rect",
            "tagged-rectangle",
            "tag-proc",
            "tagged-process",
            "flag",
            "paper-tape",
            "odd",
            "lin-doc",
            "lined-document",
            "state",
            "choice",
            "note",
            "composite",
            "icon",
            "anchor",
            "bpmn-start",
            "bpmn-intermediate",
            "bpmn-boundary",
            "bpmn-end",
            "bpmn-gateway",
            "bpmn-activity",
            "bpmn-data",
            "bpmn-data-store",
            "bpmn-annotation",
        ];
        for name in all {
            assert!(mermaid_shape(name, 1).is_ok(), "{name}");
        }
        let listed: usize = MERMAID_SHAPES.iter().map(|(n, _)| n.len()).sum();
        assert_eq!(listed, all.len(), "table and registry list agree");
        for bad in [
            "squareRect",
            "roundedRect",
            "lean_right",
            "lean_left",
            "inv_trapezoid",
            "stateStart",
            "stateEnd",
            "forkJoin",
            "rect_left_inv_arrow",
            "rectWithTitle",
            "block_arrow",
            "iconSquare",
            "imageSquare",
            "classBox",
            "square",
            "round",
        ] {
            assert!(mermaid_shape(bad, 1).is_err(), "{bad}");
        }
        for (name, want) in [
            ("trap-b", Shape::Parallelogram),
            ("trap-t", Shape::ParallelogramAlt),
            ("doc", Shape::Rect),
            ("delay", Shape::Stadium),
            ("cloud", Shape::Rounded),
            ("lin-cyl", Shape::Cylinder),
            ("hourglass", Shape::Hexagon),
            ("tri", Shape::Diamond),
            ("cross-circ", Shape::Circle),
            ("lin-rect", Shape::Subroutine),
            ("bpmn-end", Shape::DoubleCircle),
            ("bpmn-data-store", Shape::Cylinder),
            ("anchor", Shape::StateStart),
        ] {
            assert_eq!(mermaid_shape(name, 1).unwrap().0, want, "{name}");
        }
    }

    #[test]
    fn node_meta_before_its_edge_id_is_a_node() {
        // Mermaid: metadata BEFORE the edge exists mints a node, and the
        // edge still gets the id but no animation.
        let g = parse("flowchart LR\n  e1@{ animate: true }\n  A e1@--> B\n").unwrap();
        assert!(g.node_index("e1").is_some());
        assert_eq!(g.edges[0].meta.id.as_deref(), Some("e1"));
        assert_eq!(g.edges[0].meta.animation, None);
    }

    // ── linkStyle ───────────────────────────────────────────────

    fn ls_err(src: &str) -> String {
        parse(src).unwrap_err().message
    }

    #[test]
    fn link_style_index_styles_one_edge_and_strips_the_trailing_semicolon() {
        let g = parse(
            "flowchart TD\nA --> B\nB --> C\nlinkStyle 1 stroke:#ff3,stroke-width:4px,color:red;\n",
        )
        .unwrap();
        assert!(g.edges[0].meta.style.is_empty());
        let st = &g.edges[1].meta.style;
        assert_eq!(st.stroke.as_deref(), Some("#ff3"));
        assert_eq!(st.stroke_width, Some(4.0));
        assert_eq!(st.color.as_deref(), Some("red"));
        assert_eq!(st.fill, None);
    }

    #[test]
    fn link_style_counts_every_link_in_source_order() {
        // Mermaid: 0 A-C, 1 A-D, 2 B-C, 3 B-D, 4 A-S, 5 C~~~D, 6 D-E, 7 E-F,
        // though flowmaid keeps A-S apart in `sub_edges`.
        let g = parse(
            "flowchart LR\nsubgraph S\n  X\nend\nA & B --> C & D\nA --> S\nC ~~~ D\n\
             D --> E --> F\nlinkStyle 4 stroke:red\nlinkStyle 6,7 stroke:blue\n\
             linkStyle 5 stroke:green\n",
        )
        .unwrap();
        assert_eq!(g.sub_edges[0].meta.style.stroke.as_deref(), Some("red"));
        let strokes: Vec<Option<&str>> = g
            .edges
            .iter()
            .map(|e| e.meta.style.stroke.as_deref())
            .collect();
        // edges: A-C A-D B-C B-D C~~~D D-E E-F
        assert_eq!(
            strokes,
            [
                None,
                None,
                None,
                None,
                Some("green"),
                Some("blue"),
                Some("blue")
            ]
        );
        assert_eq!(g.edges[4].kind, EdgeKind::Invisible);
    }

    #[test]
    fn link_style_chain_and_mixed_fanout_order() {
        // `A --> B & C --> D`: A-B, A-C, B-D, C-D.
        let g = parse("flowchart LR\nA --> B & C --> D\nlinkStyle 2 stroke:red\n").unwrap();
        let styled: Vec<(String, String)> = g
            .edges
            .iter()
            .filter(|e| e.meta.style.stroke.is_some())
            .map(|e| (g.nodes[e.from].id.clone(), g.nodes[e.to].id.clone()))
            .collect();
        assert_eq!(styled, [("B".to_string(), "D".to_string())]);
        // Repeated links each get an index.
        let g = parse("flowchart LR\nA --> B\nA --> B\nlinkStyle 1 stroke:red\n").unwrap();
        assert!(g.edges[0].meta.style.is_empty());
        assert_eq!(g.edges[1].meta.style.stroke.as_deref(), Some("red"));
    }

    #[test]
    fn link_style_index_out_of_bounds_uses_mermaids_message() {
        let m = ls_err("flowchart TD\nA --> B\nlinkStyle 1 stroke:red\n");
        assert!(
            m.contains("The index 1 for linkStyle is out of bounds")
                && m.contains("between 0 and 0."),
            "{m}"
        );
        // A forward reference counts links defined so far: none.
        let e = parse("flowchart TD\nlinkStyle 0 stroke:red\nA --> B\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("between 0 and -1."), "{}", e.message);
        // One bad index fails the whole statement; the interpolate form
        // gets the same message (mermaid throws a TypeError there).
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle 0,9 stroke:red\n").contains("index 9"));
        assert!(
            ls_err("flowchart TD\nA --> B\nlinkStyle 3 interpolate basis\n")
                .contains("The index 3 for linkStyle is out of bounds")
        );
        assert!(
            ls_err("flowchart TD\nA --> B\nlinkStyle 99999999999999999999999 stroke:red\n")
                .contains("out of bounds")
        );
    }

    #[test]
    fn link_style_malformed_statements_are_errors() {
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle 0\n").contains("needs styles"));
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle default\n").contains("needs styles"));
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle 0, 1 stroke:red\n").contains("no spaces"));
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle x stroke:red\n").contains("index list"));
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle -1 stroke:red\n").contains("index list"));
        assert!(ls_err("flowchart TD\nA --> B\nlinkStyle 0 interpolate\n").contains("curve name"));
        assert!(
            ls_err("flowchart TD\nA --> B\nlinkStyle 0 stroke-width:2em\n")
                .contains("invalid stroke-width")
        );
        // Negative, non-finite and absurd widths would write invalid
        // or 300-digit SVG numbers; node `style` shares the check.
        for w in ["-5px", "-0", "NaN", "inf", "1e308px", "1001px"] {
            for src in [
                format!("flowchart TD\nA --> B\nlinkStyle 0 stroke-width:{w}\n"),
                format!("flowchart TD\nA --> B\nstyle A stroke-width:{w}\n"),
                format!("flowchart TD\nA --> B\nclassDef c stroke-width:{w}\n"),
            ] {
                assert!(ls_err(&src).contains("invalid stroke-width"), "{src}");
            }
        }
        for (w, want) in [("0", 0.0), ("0.5px", 0.5), ("1000px", 1000.0)] {
            let g = parse(&format!(
                "flowchart TD\nA --> B\nlinkStyle 0 stroke-width:{w}\nstyle A stroke-width:{w}\n"
            ))
            .unwrap();
            assert_eq!(g.edges[0].meta.style.stroke_width, Some(want));
            assert_eq!(g.nodes[0].style.stroke_width, Some(want));
        }
        assert!(
            ls_err("flowchart TD\nA --> B\nlinkStyle 0 stroke:rgb(1,2\n").contains("unbalanced")
        );
        // `0,0` is harmless; `1,2,7`-style lists without spaces are fine.
        assert!(parse("flowchart TD\nA --> B\nlinkStyle 0,0 stroke:red\n").is_ok());
    }

    #[test]
    fn link_style_replaces_per_index_and_merges_over_default() {
        // A second `linkStyle 0` replaces the first (no merge).
        let g =
            parse("flowchart TD\nA --> B\nlinkStyle 0 stroke:red\nlinkStyle 0 stroke-width:4px\n")
                .unwrap();
        let st = &g.edges[0].meta.style;
        assert_eq!((st.stroke.as_deref(), st.stroke_width), (None, Some(4.0)));
        // Default reaches later edges; per-index wins per property.
        let g = parse(
            "flowchart TD\nlinkStyle default stroke:#888,stroke-width:2px\nA --> B --> C\n\
             linkStyle 1 stroke:#e11d48\n",
        )
        .unwrap();
        let (a, b) = (&g.edges[0].meta.style, &g.edges[1].meta.style);
        assert_eq!(
            (a.stroke.as_deref(), a.stroke_width),
            (Some("#888"), Some(2.0))
        );
        assert_eq!(
            (b.stroke.as_deref(), b.stroke_width),
            (Some("#e11d48"), Some(2.0))
        );
        // A later default replaces the earlier one.
        let g = parse(
            "flowchart TD\nA --> B\nlinkStyle default stroke:red\nlinkStyle default color:blue\n",
        )
        .unwrap();
        let st = &g.edges[0].meta.style;
        assert_eq!(
            (st.stroke.as_deref(), st.color.as_deref()),
            (None, Some("blue"))
        );
        // Mermaid appends `fill:none` to an index style without fill, so
        // it clears a default fill; the default alone keeps it.
        let g = parse(
            "flowchart TD\nA --> B --> C\nlinkStyle default fill:#f00\nlinkStyle 1 stroke:red\n",
        )
        .unwrap();
        assert_eq!(g.edges[0].meta.style.fill.as_deref(), Some("#f00"));
        assert_eq!(g.edges[1].meta.style.fill, None);
    }

    #[test]
    fn link_style_property_parsing() {
        let st = |src: &str| parse(src).unwrap().edges[0].meta.style.clone();
        // Dash with spaces; dash with a comma keeps mermaid's `5`.
        let a = st("flowchart TD\nA --> B\nlinkStyle 0 stroke-dasharray: 5 5\n");
        assert_eq!(a.stroke_dasharray.as_deref(), Some("5 5"));
        let a = st("flowchart TD\nA --> B\nlinkStyle 0 stroke-dasharray:5,5\n");
        assert_eq!(a.stroke_dasharray.as_deref(), Some("5"));
        // rgb() with commas is a lenient superset of mermaid.
        let a = st("flowchart TD\nA --> B\nlinkStyle 0 stroke:rgb(255,0,0)\n");
        assert_eq!(a.stroke.as_deref(), Some("rgb(255,0,0)"));
        // Explicit fill, unknown keys ignored.
        let a = st("flowchart TD\nA --> B\nlinkStyle 0 fill:#fde68a,stroke:#b45309,opacity:0.5\n");
        assert_eq!(a.fill.as_deref(), Some("#fde68a"));
        assert_eq!(a.stroke.as_deref(), Some("#b45309"));
        // A style naming nothing known is accepted and draws unstyled.
        assert!(st("flowchart TD\nA --> B\nlinkStyle 0 foo\n").is_empty());
    }

    #[test]
    fn link_style_interpolate_sets_the_curve() {
        let g = parse(
            "flowchart TD\nA --> B --> C\nlinkStyle default interpolate linear stroke:red\n\
             linkStyle 0 interpolate basis\n",
        )
        .unwrap();
        assert_eq!(g.edges[0].meta.curve, Some(EdgeCurve::Basis));
        assert_eq!(g.edges[1].meta.curve, Some(EdgeCurve::Linear));
        assert_eq!(g.edges[1].meta.style.stroke.as_deref(), Some("red"));
        // The interpolate-only form leaves the style alone.
        let g = parse(
            "flowchart TD\nA --> B\nlinkStyle 0 stroke:red\nlinkStyle 0 interpolate stepAfter\n",
        )
        .unwrap();
        assert_eq!(g.edges[0].meta.curve, Some(EdgeCurve::StepAfter));
        assert_eq!(g.edges[0].meta.style.stroke.as_deref(), Some("red"));
        // Unknown names fall back to basis, like mermaid.
        let g = parse("flowchart TD\nA --> B\nlinkStyle default interpolate wiggly\n").unwrap();
        assert_eq!(g.edges[0].meta.curve, Some(EdgeCurve::Basis));
        // `@{ curve }` wins over linkStyle.
        let g =
            parse("flowchart TD\nA e1@--> B\ne1@{ curve: step }\nlinkStyle 0 interpolate linear\n")
                .unwrap();
        assert_eq!(g.edges[0].meta.curve, Some(EdgeCurve::Step));
        // No linkStyle: no curve.
        assert_eq!(
            parse("flowchart TD\nA --> B\n").unwrap().edges[0]
                .meta
                .curve,
            None
        );
    }

    #[test]
    fn link_style_keyword_is_exact_case_and_named_nodes_stay_edges() {
        let g = parse("flowchart TD\nlinkStyle --> B\nlinkStyle <-- t --> B\n").unwrap();
        assert_eq!(g.edges.len(), 2);
        assert_eq!(g.nodes[g.edges[0].from].id, "linkStyle");
        // `linkstyle` is no statement in mermaid either.
        assert!(parse("flowchart TD\nA --> B\nlinkstyle 0 stroke:red\n").is_err());
        // Not a statement in state diagrams.
        assert!(parse_document("stateDiagram-v2\nA --> B\nlinkStyle 0 stroke:red\n").is_err());
    }

    // ── click ───────────────────────────────────────────────────

    fn link_of(src: &str, id: &str) -> Option<crate::model::NodeLink> {
        let g = parse(src).unwrap_or_else(|e| panic!("{e}\n{src}"));
        g.node_index(id).and_then(|n| g.links.get(&n).cloned())
    }

    #[test]
    fn click_href_forms_parse_in_mermaids_order() {
        let l = link_of(
            "flowchart LR\nA-->B\nclick A \"https://example.com\"\n",
            "A",
        )
        .unwrap();
        assert_eq!(l.href.as_deref(), Some("https://example.com"));
        assert_eq!((l.target, l.tooltip.as_deref()), (None, None));
        let l = link_of(
            "flowchart LR\nA-->B\nclick A href \"https://example.com\" \"Go to docs\" _blank\n",
            "A",
        )
        .unwrap();
        assert_eq!(l.tooltip.as_deref(), Some("Go to docs"));
        assert_eq!(l.target, Some(LinkTarget::Blank));
        let l = link_of(
            "flowchart LR\nA-->B\nclick A \"https://example.com\" _self\n",
            "A",
        )
        .unwrap();
        assert_eq!(l.target, Some(LinkTarget::SelfFrame));
        let l = link_of(
            "flowchart LR\nA-->B\nclick B \"https://example.com\" \"tip\"\n",
            "B",
        )
        .unwrap();
        assert_eq!(l.tooltip.as_deref(), Some("tip"));
        for t in ["_parent", "_top"] {
            let l = link_of(
                &format!("flowchart LR\nA-->B\nclick A href \"u\" {t}\n"),
                "A",
            );
            assert_eq!(l.unwrap().target.unwrap().as_str(), t);
        }
        // Extra whitespace is accepted (mermaid wants exactly one).
        let l = link_of(
            "flowchart LR\nA-->B\nclick A  href  \"https://x.example\"\n",
            "A",
        );
        assert!(l.unwrap().href.is_some());
    }

    #[test]
    fn click_callbacks_are_accepted_and_keep_their_tooltip() {
        let l = link_of("flowchart LR\nA-->B\nclick A callback\n", "A").unwrap();
        assert_eq!((l.href, l.callback.as_deref()), (None, Some("callback")));
        let l = link_of(
            "flowchart LR\nA-->B\nclick A callback \"Tooltip for a callback\"\n",
            "A",
        )
        .unwrap();
        assert_eq!(l.tooltip.as_deref(), Some("Tooltip for a callback"));
        let l = link_of("flowchart LR\nA-->B\nclick A call callback()\n", "A").unwrap();
        assert_eq!(l.callback.as_deref(), Some("callback"));
        let l = link_of(
            "flowchart LR\nA-->B\nclick A call fn(\"a,b\", 2) \"tt\"\n",
            "A",
        )
        .unwrap();
        assert_eq!(
            (l.callback.as_deref(), l.tooltip.as_deref()),
            (Some("fn"), Some("tt"))
        );
        let l = link_of("flowchart LR\nA-->B\nclick A call fn\n", "A").unwrap();
        assert_eq!(l.callback.as_deref(), Some("fn"));
        // `callback` and `caller` are names, not `call`.
        for name in ["caller", "callback"] {
            let l = link_of(&format!("flowchart LR\nA-->B\nclick A {name}\n"), "A").unwrap();
            assert_eq!(l.callback.as_deref(), Some(name));
        }
    }

    #[test]
    fn click_targets_ids_like_mermaid() {
        // Several ids; a `;` separator; a `;` inside the url.
        let g = parse("flowchart LR\nA-->B\nC-->D\nclick A,C \"https://example.com\"\n").unwrap();
        assert_eq!(g.links.len(), 2);
        let g = parse(
            "flowchart LR\nA-->B\nclick A \"https://x.example\"; click B \"https://y.example\"\n",
        )
        .unwrap();
        assert_eq!(g.links.len(), 2);
        let l = link_of(
            "flowchart LR\nA-->B\nclick A \"https://x.example/a;b\"\n",
            "A",
        )
        .unwrap();
        assert_eq!(l.href.as_deref(), Some("https://x.example/a;b"));
        let l = link_of("flowchart LR\nA-->B\nclick A \"https://x/%%41\"\n", "A").unwrap();
        assert_eq!(l.href.as_deref(), Some("https://x/%%41"));
        // Unknown ids and subgraph ids: accepted, nothing created.
        let g = parse("flowchart LR\nA-->B\nclick Z \"https://example.com\"\n").unwrap();
        assert_eq!((g.nodes.len(), g.links.len()), (2, 0));
        let g = parse("flowchart LR\nsubgraph S [Box]\nA\nend\nclick S \"https://example.com\"\n")
            .unwrap();
        assert!(g.node_index("S").is_none() && g.links.is_empty());
        // An edge id is accepted too.
        let g = parse("flowchart LR\nA e1@--> B\nclick e1 \"https://example.com\"\n").unwrap();
        assert!(g.node_index("e1").is_none() && g.links.is_empty());
        // Before the node exists: the link is dropped, the tooltip kept.
        let l = link_of(
            "flowchart LR\nclick A \"https://example.com\" \"tt\"\nA-->B\n",
            "A",
        )
        .unwrap();
        assert_eq!((l.href, l.tooltip.as_deref()), (None, Some("tt")));
        // Last link and last tooltip win independently.
        let l = link_of(
            "flowchart LR\nA-->B\nclick A \"https://u1\" \"t1\"\nclick A cb \"tt\"\nclick A \"https://u2\"\n",
            "A",
        )
        .unwrap();
        assert_eq!(
            (l.href.as_deref(), l.tooltip.as_deref()),
            (Some("https://u2"), Some("tt"))
        );
        // A whitespace-only url clears the link; empty quotes are lenient.
        let l = link_of(
            "flowchart LR\nA-->B\nclick A \"https://u1\"\nclick A \"  \"\n",
            "A",
        );
        assert!(l.is_none());
        assert!(link_of("flowchart LR\nA-->B\nclick A \"\"\n", "A").is_none());
    }

    #[test]
    fn click_rejects_dangerous_urls() {
        for url in [
            "javascript:alert(1)",
            " JaVaScRiPt:alert(1)",
            "java\tscript:alert(1)",
            "%6Aavascript:alert(1)",
            "javascript#58;alert(1)",
            "&#106;avascript:alert(1)",
            "&#x6A;avascript:alert(1)",
            "javascript&colon;alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "vbscript:msgbox(1)",
            "\u{200B}javascript:alert(1)",
        ] {
            let src = format!("flowchart LR\nA-->B\nclick A \"{url}\" \"tip\"\n");
            let l = link_of(&src, "A").unwrap();
            assert_eq!(l.href, None, "{url:?}");
            assert_eq!(l.tooltip.as_deref(), Some("tip"));
        }
        for url in [
            "/docs",
            "./a",
            "../b",
            "#anchor",
            "?q=1",
            "www.github.com",
            "mailto:x@example.com",
            "https://x.com/a?b=1&c=2",
            "vscode://file/x",
        ] {
            let src = format!("flowchart LR\nA-->B\nclick A \"{url}\"\n");
            assert_eq!(
                link_of(&src, "A").unwrap().href.as_deref(),
                Some(url),
                "{url}"
            );
        }
    }

    #[test]
    fn click_malformed_statements_are_line_numbered_errors() {
        let e = |src: &str| {
            let e = parse(src).unwrap_err();
            assert_eq!(e.line, 3, "{src}");
            e.message
        };
        assert!(
            e("flowchart LR\nA-->B\nclick A \"https://x.example\" _new\n").contains("link target")
        );
        assert!(
            e("flowchart LR\nA-->B\nclick A \"https://x.example\" _blank \"tt\"\n")
                .contains("tooltip must come before")
        );
        assert!(e("flowchart LR\nA-->B\nclick A\n").contains("expected a url"));
        assert!(e("flowchart LR\nA-->B\nclick A href\n").contains("quoted url after 'href'"));
        assert!(e("flowchart LR\nA-->B\nclick A _blank\n").contains("link target '_blank'"));
        assert!(e("flowchart LR\nA-->B\nclick A \"u\" blank\n").contains("link target"));
        assert!(e("flowchart LR\nA-->B\nclick A \"u\n").contains("unclosed"));
        assert!(e("flowchart LR\nA-->B\nclick A call fn(1\n").contains("never closed"));
        assert!(e("flowchart LR\nA-->B\nclick A cb \"t\" extra\n").contains("unexpected"));
        assert!(e("flowchart LR\nA-->B\nclick A call\n").contains("function name"));
    }

    #[test]
    fn click_keyword_is_exact_case_and_named_nodes_stay_edges() {
        let g = parse("flowchart LR\nclick --> B\n").unwrap();
        assert_eq!(g.nodes[g.edges[0].from].id, "click");
        let g = parse("flowchart LR\nclick -- go --> B\n").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("go"));
        let e = parse("flowchart LR\nA-->B\nClick A \"u\"\n").unwrap_err();
        assert!(
            e.message.contains("unknown edge operator near: 'A \"u\"'"),
            "{}",
            e.message
        );
        // stateDiagram has no click grammar.
        assert!(parse_document("stateDiagram-v2\nA --> B\nclick A \"u\"\n").is_err());
    }

    #[test]
    fn sanitize_href_ports_braintree() {
        use crate::model::sanitize_href;
        assert_eq!(sanitize_href("  https://x  ").as_deref(), Some("https://x"));
        assert_eq!(sanitize_href(""), None);
        assert_eq!(sanitize_href("\u{0}\u{1}"), None);
        // Double-encoded percent.
        assert_eq!(sanitize_href("%256Aavascript:alert(1)"), None);
        assert_eq!(sanitize_href("&#0000106avascript:alert(1)"), None);
        assert_eq!(sanitize_href("&#106avascript:alert(1)"), None);
        // A colon later in a relative-looking url still counts, as in
        // braintree (`database.html?x=a:b` reads as a `data…` scheme).
        assert_eq!(sanitize_href("database.html?x=a:b"), None);
        assert_eq!(
            sanitize_href("/javascript:x").as_deref(),
            Some("/javascript:x")
        );
        assert_eq!(sanitize_href("%zz").as_deref(), Some("%zz"));
        // Named whitespace entities hide a scheme too (braintree's
        // `&tab;` / `&newline;`, any case).
        for u in [
            "java&tab;script:alert(1)",
            "java&newline;script:alert(1)",
            "java&Tab;script:alert(1)",
            "java&NewLine;script:alert(1)",
        ] {
            assert_eq!(sanitize_href(u), None, "{u}");
        }
    }

    // ── markdown strings ────────────────────────────────────────

    #[test]
    fn markdown_node_labels_lower_to_label_markup() {
        let g = parse("flowchart LR\n  A[\"`This **is** _Markdown_`\"] --> B\n").unwrap();
        assert_eq!(g.nodes[0].label, "This <b>is</b> <i>Markdown</i>");
        // Spans physical lines (the baseline said "unclosed label quote").
        let g = parse(
            "flowchart LR\n  newLines[\"`Line1\n    Line 2\n    Line 3`\"]\n  newLines --> X\n",
        )
        .unwrap();
        assert_eq!(g.nodes[0].label, "Line1\nLine 2\nLine 3");
        assert_eq!(g.edges.len(), 1);
        let g = parse("flowchart LR\n  A[\"`***both*** __strong__ *em* _em2_`\"]\n").unwrap();
        assert_eq!(
            g.nodes[0].label,
            "<b><i>both</i></b> <b>strong</b> <i>em</i> <i>em2</i>"
        );
        let src = "snake_case_name and 2 * 3 * 4 and **unclosed";
        let g = parse(&format!("flowchart LR\n  A[\"`{src}`\"]\n")).unwrap();
        // No emphasis anywhere (the long line wraps, so compare words).
        assert_eq!(g.nodes[0].label.replace('\n', " "), src);
        let g = parse("flowchart LR\n  A[\"`\\*not italic\\* #42;also not#42;`\"]\n").unwrap();
        assert_eq!(g.nodes[0].label, "*not italic* *also not*");
        let g = parse("flowchart LR\n  A[\"`# heading\n  - item`\"]\n").unwrap();
        assert_eq!(g.nodes[0].label, "# heading\n- item");
        let g = parse("flowchart LR\n  A[\"`a\n\n\n  b`\"]\n").unwrap();
        assert_eq!(g.nodes[0].label, "a\nb");
        let g = parse("flowchart LR\n  A[\"`one<br/>two **b**`\"]\n").unwrap();
        assert_eq!(g.nodes[0].label, "one\ntwo <b>b</b>");
        // Every shape opener takes one.
        let g = parse("flowchart LR\n  a(\"`**r**`\") --> b{{\"`**h**`\"}} --> c((\"`**c**`\"))\n")
            .unwrap();
        assert!(
            g.nodes.iter().all(|n| n.label.starts_with("<b>")),
            "{:?}",
            g.nodes
        );
    }

    #[test]
    fn markdown_node_labels_wrap() {
        let g = parse(
            "flowchart LR\n  A[\"`The quick brown fox jumps over the lazy dog and keeps running far away`\"]\n",
        )
        .unwrap();
        let lines: Vec<&str> = g.nodes[0].label.split('\n').collect();
        assert!(lines.len() >= 3, "{lines:?}");
        assert!(
            lines.iter().all(|l| crate::layout::text_width(l) <= 175.0),
            "{lines:?}"
        );
        let (_, h) = crate::layout::intrinsic_size(&g.nodes[0]);
        assert_eq!(
            h,
            crate::layout::BASE_H + (lines.len() - 1) as f64 * crate::layout::LINE_H
        );
        let w = "Supercalifragilisticexpialidociousword";
        let g = parse(&format!("flowchart LR\n  A[\"`{w}`\"]\n")).unwrap();
        assert_eq!(g.nodes[0].label, w);
    }

    #[test]
    fn markdown_needs_the_quote_backtick_opener() {
        let g = parse("flowchart LR\n  A[\" `x` \"]\n  B[`y`]\n  C[\"#96;z#96;\"]\n").unwrap();
        let labels: Vec<&str> = g.nodes.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(labels, ["`x`", "`y`", "`z`"]);
        let g = parse("flowchart LR\n  A[**x**]\n").unwrap();
        assert_eq!(g.nodes[0].label, "**x**");
    }

    #[test]
    fn markdown_edge_labels_and_subgraph_titles() {
        let g = parse(
            "flowchart LR\n  A -->|\"`**pipe** label`\"| B\n  A -- \"`*inline* label`\" --> C\n",
        )
        .unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("<b>pipe</b> label"));
        assert_eq!(g.edges[1].label.as_deref(), Some("<i>inline</i> label"));
        // A `|` inside a markdown pipe label is text.
        let g = parse("flowchart LR\n  A -->|\"`a|b`\"| B\n").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("a|b"));
        // Phase 1: edge labels are one line.
        let g = parse("flowchart LR\n  A -- \"`two\n  lines`\" --> B\n").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("two lines"));
        let g =
            parse("flowchart LR\n  subgraph s1 [\"`*Italic* title`\"]\n    A\n  end\n").unwrap();
        assert_eq!(g.subgraphs[0].title, "<i>Italic</i> title");
        let g = parse(
            "flowchart LR\nsubgraph \"One\"\n  a(\"`The **cat**\n  in the hat`\") -- \"edge label\" --> \
             b{{\"`The **dog** in the hog`\"}}\nend\nsubgraph \"`**Two**`\"\n  c(\"`The **cat**\n  \
             in the hat`\") -- \"`Bold **edge label**`\" --> d(\"The dog in the hog\")\nend\n",
        )
        .unwrap();
        assert_eq!(g.nodes[0].label, "The <b>cat</b>\nin the hat");
        assert_eq!(g.nodes[1].label, "The <b>dog</b> in the hog");
        assert_eq!(g.edges[1].label.as_deref(), Some("Bold <b>edge label</b>"));
        assert_eq!(g.subgraphs[1].id, "**Two**");
        assert_eq!(g.subgraphs[1].title, "<b>Two</b>");
        assert_eq!(g.subgraphs[1].nodes.len(), 2);
    }

    #[test]
    fn markdown_strings_protect_semicolons_and_report_errors() {
        let g = parse("flowchart TD\n  A[\"`x;y`\"] --> B; B --> C\n").unwrap();
        assert_eq!(g.nodes[0].label, "x;y");
        assert_eq!(g.edges.len(), 2);
        let e = parse("flowchart LR\n  A[\"`bad ` tick`\"]\n").unwrap_err();
        assert!(e.message.contains("#96;"), "{}", e.message);
        assert_eq!(e.line, 2);
        let e = parse("flowchart LR\n  A[\"`never closed\n  B --> C\n").unwrap_err();
        assert!(e.message.contains("never closed"), "{}", e.message);
        assert_eq!(e.line, 2);
        // Empty is lenient (mermaid rejects it).
        let g = parse("flowchart LR\n  A[\"``\"]\n").unwrap();
        assert_eq!(g.nodes[0].label, "");
    }

    // ── reviewer follow-ups (#syntax-gaps) ─────────────────────────

    #[test]
    fn class_def_ignores_bare_comma_items_like_mermaid() {
        // Verbatim from mermaid's flowchart.md ("Using classDef
        // Statements for Animations").
        let src = "flowchart LR\n  A e1@--> B\n  classDef animate stroke-dasharray: 9,5,stroke-dashoffset: 900,animation: dash 25s linear infinite;\n  class e1 animate\n";
        let g = parse(src).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(g.edges.len(), 1);
        let g = parse(
            "flowchart LR\n  classDef c fill:#f00,5,color:#fff\n  A:::c\n  style A stroke:#00f,7\n",
        )
        .unwrap();
        let st = &g.nodes[0].style;
        assert_eq!(
            (
                st.fill.as_deref(),
                st.color.as_deref(),
                st.stroke.as_deref()
            ),
            (Some("#f00"), Some("#fff"), Some("#00f"))
        );
        // A bare item with no property before it, and an unbalanced
        // '(', are still errors.
        assert!(parse("flowchart LR\n  classDef c 5,fill:#f00\n").is_err());
        assert!(parse("flowchart LR\n  classDef c fill:rgb(1,2\n").is_err());
    }

    #[test]
    fn acc_inside_multi_line_constructs_is_label_or_yaml_text() {
        // (a) a statement after `;` on the line closing a markdown string.
        let (t, _) = acc("flowchart LR\nA[\"`a\nb`\"] --> B; accTitle: z\n");
        assert_eq!(t.as_deref(), Some("z"));
        let g = parse("flowchart LR\nA[\"`a\nb`\"] --> B; accTitle: z\n").unwrap();
        assert_eq!((g.nodes.len(), g.edges.len()), (2, 1));
        // (b) a YAML key inside a multi-line `@{` block is not acc.
        let src = "flowchart LR\nA@{\n  label: foo\n  accTitle: x\n}\n";
        assert_eq!(acc(src), (None, None));
        // (c) a markdown label line starting with an acc keyword is
        // label text.
        let src = "flowchart LR\nA[\"`first\naccDescr { hi }`\"]\n";
        assert_eq!(acc(src), (None, None));
        assert_eq!(parse(src).unwrap().nodes[0].label, "first\naccDescr { hi }");
        let src = "flowchart LR\nA[\"`first\naccTitle: x`\"]\n";
        assert_eq!(acc(src), (None, None));
        assert_eq!(parse(src).unwrap().nodes[0].label, "first\naccTitle: x");
        // A comment line holding `@{` never opens a block.
        let src = "flowchart LR\n%% A@{\naccTitle: t\nA --> B\n";
        assert_eq!(acc(src).0.as_deref(), Some("t"));
        // After the construct closes, acc lines work again.
        let src = "flowchart LR\nA@{\n  label: foo\n}\naccTitle: after\n";
        assert_eq!(acc(src).0.as_deref(), Some("after"));
    }

    #[test]
    fn odd_and_trapezoid_bracket_shapes() {
        let g = parse("flowchart TD\n  id1>This is the text in the box]\n  A[/Christmas\\]\n  B[\\Go shopping/]\n  C[/a\\b/]\n  D[\\\"x/]\"/]\n").unwrap();
        let got: Vec<(&str, Shape)> = g
            .nodes
            .iter()
            .map(|n| (n.label.as_str(), n.shape))
            .collect();
        assert_eq!(
            got,
            [
                ("This is the text in the box", Shape::Rect),
                ("Christmas", Shape::Parallelogram),
                ("Go shopping", Shape::ParallelogramAlt),
                ("a\\b", Shape::Parallelogram),
                ("x/]", Shape::ParallelogramAlt),
            ]
        );
        let g = parse("flowchart LR\n  A>odd] --> B[/t\\] --> C\n").unwrap();
        assert_eq!((g.nodes.len(), g.edges.len()), (3, 2));
        // An unclosed label names the opener's own closer.
        let e = parse("flowchart TD\n  A[\\x\n").unwrap_err();
        assert!(e.message.contains("'\\]'"), "{}", e.message);
        let e = parse("flowchart TD\n  A>x\n").unwrap_err();
        assert!(e.message.contains("']'"), "{}", e.message);
    }

    #[test]
    fn at_brace_inside_label_text_does_not_open_shape_data() {
        for stmt in [
            "A[x @{ y] --> B",
            "A -->|x @{ y| B",
            "A -- x @{ y --> B",
            "A -- x@{ y --> B",
            "A -. x @{ y .-> B",
            "A == x @{ y ==> B",
            "A(use @{ here) --> B",
            "A{is @{ ok} --> B",
            "A[template @{name] --> B",
        ] {
            let g = parse(&format!("flowchart TD\n  {stmt}\n  B --> C\n  C --> D\n"))
                .unwrap_or_else(|e| panic!("{stmt}: {e}"));
            assert_eq!(g.edges.len(), 3, "{stmt}");
        }
        let g = parse("flowchart TD\n  subgraph S[cfg @{ x]\n  A\n  end\n").unwrap();
        assert_eq!(g.subgraphs.len(), 1);
        // The next line is not swallowed into a phantom block.
        let g = parse("flowchart TD\n  A[use @{] --> B\n  C[then }] --> D\n").unwrap();
        assert_eq!((g.nodes.len(), g.edges.len()), (4, 2));
        assert_eq!(node(&g, "A").label, "use @{");
        // Real shape data after a label still opens (and may span lines).
        let g = parse("flowchart TD\n  A -- t @{ --> B@{\n    shape: cyl\n  }\n").unwrap();
        assert_eq!(node(&g, "B").shape, Shape::Cylinder);
        assert_eq!(g.edges[0].label.as_deref(), Some("t @{"));
    }

    #[test]
    fn block_scalars_follow_js_yaml() {
        // Folded: a more-indented line keeps its own line and indent
        // (a plain label shows it; markdown drops line indentation).
        let g = parse(
            "flowchart LR\n  A@{\n    labelType: text\n    label: >\n      a\n        b\n      c\n  }\n",
        )
        .unwrap();
        assert_eq!(node(&g, "A").label, "a\n  b\nc");
        let g =
            parse("flowchart LR\n  A@{\n    label: >\n      a\n        b\n      c\n  }\n").unwrap();
        assert_eq!(node(&g, "A").label, "a\nb\nc");
        // Literal: a line indented less than the first is an error.
        let e = parse("flowchart LR\n  A@{\n    label: |\n        a\n      b\n  }\n").unwrap_err();
        assert!(e.message.contains("bad indentation"), "{}", e.message);
    }
}
