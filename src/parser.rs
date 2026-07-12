//! Hand-written parser for Mermaid-like flowchart syntax.
//!
//! Supported:
//! - Header:  `flowchart TD|TB|LR|RL|BT`  or  `graph ...`
//! - Nodes:   `A`, `A[text]`, `A(text)`, `A([text])`, `A{text}`, `A((text))`
//! - Edges:   `-->`, `---`, `-.->`, `==>`, with labels `-->|text|`
//! - Chains:  `A --> B --> C`, `;` separator, `%%` comments

use crate::model::{Direction, EdgeKind, Graph, Shape};

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

/// Strip wrapping double quotes from a label, Mermaid-style: `A["odd text"]`.
fn clean_label(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

pub fn parse(source: &str) -> Result<Graph, ParseError> {
    let mut g = Graph::default();
    let mut header_seen = false;

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_seen {
            header_seen = true;
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
        parse_statement(&mut g, line, lineno)?;
    }
    Ok(g)
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

fn parse_statement(g: &mut Graph, line: &str, lineno: usize) -> Result<(), ParseError> {
    let mut cur = Cur::new(line);
    let mut prev = parse_node(&mut cur, g, lineno)?;
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
            prev = parse_node(&mut cur, g, lineno)?;
            continue;
        }
        let kind = parse_edge_op(&mut cur).ok_or_else(|| {
            err(
                lineno,
                format!("unknown edge operator near: '{}'", cur.rest()),
            )
        })?;
        cur.skip_ws();
        let label = if cur.eat("|") {
            let l = cur.take_until("|").ok_or_else(|| {
                err(lineno, "edge label opened with '|' but never closed".to_string())
            })?;
            Some(clean_label(&l))
        } else {
            None
        };
        let next = parse_node(&mut cur, g, lineno)?;
        g.add_edge(prev, next, label, kind);
        prev = next;
    }
    Ok(())
}

fn parse_node(cur: &mut Cur<'_>, g: &mut Graph, lineno: usize) -> Result<usize, ParseError> {
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

    // Check order matters: two-character openers first.
    let parsed: Option<(Shape, String)> = if cur.eat("((") {
        Some((Shape::Circle, close(cur, "))", lineno)?))
    } else if cur.eat("([") {
        Some((Shape::Stadium, close(cur, "])", lineno)?))
    } else if cur.eat("[") {
        Some((Shape::Rect, close(cur, "]", lineno)?))
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
    Ok(g.ensure_node(&id, label, shape))
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

    // Dotted: -.->  or  -..->
    if rest.starts_with("-.") {
        let after = &rest[2..];
        let dots = after.chars().take_while(|&c| c == '.').count();
        let tail = &after[dots..];
        if tail.starts_with("->") {
            cur.pos += 2 + dots + 2;
            return Some(EdgeKind::Dotted);
        }
        return None;
    }

    // Thick: ==>  or  ===>
    if rest.starts_with("==") {
        let eqs = rest.chars().take_while(|&c| c == '=').count();
        let tail = &rest[eqs..];
        if tail.starts_with('>') {
            cur.pos += eqs + 1;
            return Some(EdgeKind::Thick);
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
    fn keyword_like_id_is_not_a_header() {
        // Previously: "graphics" was mistaken for the "graph" header
        // plus direction "ics[...]" -> error.
        let g = parse("graphics[Graphics] --> B").unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[0].id, "graphics");
        assert_eq!(g.direction, crate::model::Direction::TD);
    }
}
