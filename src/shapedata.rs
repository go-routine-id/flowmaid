//! Mermaid's `@{ … }` shape-data blocks (`A@{ shape: cyl }`,
//! `e1@{ animate: true }`): finding where a block ends, and reading the
//! small YAML subset its body is written in.
//!
//! Mermaid hands the body to js-yaml with the JSON schema: a body with no
//! newline is wrapped in braces and read as a flow mapping
//! (`{ shape: cyl, label: "DB" }`), a body that spans lines is read as a
//! block mapping. flowmaid has no dependencies, so this is a hand-written
//! reader for exactly what shape data needs — one mapping of scalar
//! values — with js-yaml's outcomes on the edge cases (verified against
//! js-yaml 4.3): a missing comma, a duplicate key and bad indentation are
//! errors; `shape:cyl` is one key with a null value; a trailing `,` in
//! the block form is part of the value. Nested collections (`[1, 2]`)
//! are skipped as opaque values, since no shape-data key uses one.

/// One shape-data value, typed the way js-yaml's JSON schema types it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Yaml {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    /// A flow sequence or mapping (`[1, 2]`), kept opaque.
    Collection,
}

impl Yaml {
    /// JavaScript truthiness, which is how mermaid tests these values
    /// (`if (doc.label)`, `if (edge.animate)`).
    pub(crate) fn truthy(&self) -> bool {
        match self {
            Yaml::Null => false,
            Yaml::Bool(b) => *b,
            Yaml::Num(n) => *n != 0.0 && !n.is_nan(),
            Yaml::Str(s) => !s.is_empty(),
            Yaml::Collection => true,
        }
    }

    /// The value as JavaScript's `String(v)` would print it; `None` for
    /// null and collections.
    pub(crate) fn text(&self) -> Option<String> {
        match self {
            Yaml::Null | Yaml::Collection => None,
            Yaml::Bool(b) => Some(b.to_string()),
            Yaml::Num(n) if n.is_infinite() => {
                Some(if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string())
            }
            Yaml::Num(n) if n.is_nan() => Some("NaN".to_string()),
            Yaml::Num(n) => Some(n.to_string()),
            Yaml::Str(s) => Some(s.clone()),
        }
    }
}

/// Byte offset of the `}` closing a shape-data body that starts at the
/// beginning of `s` (just past the `@{`), or `None` when it never
/// closes. Mirrors mermaid's lexer: only double quotes protect a `}`
/// (plain quote parity, no backslash escape) — a single-quoted `'a}b'`
/// closes at its `}`.
pub(crate) fn body_end(s: &str) -> Option<usize> {
    let mut in_quote = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            '}' if !in_quote => return Some(i),
            _ => {}
        }
    }
    None
}

/// Byte offset of the first `@{` in `s` that opens a shape-data block.
/// Mermaid lexes `@{` only in its INITIAL state, so one inside label
/// text is plain text: a double-quoted string, a bracket shape label
/// (`A[x @{ y]`, `A(…)`, `A{…}`, subgraph titles), an edge-label
/// `|…|` and an inline edge label (`A -- x @{ y --> B`) are all
/// skipped — the same contexts [`crate::parser`]'s statement splitter
/// protects.
pub(crate) fn find_open(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let (mut in_quote, mut in_pipe) = (false, false);
    let mut i = 0;
    while let Some(c) = s[i..].chars().next() {
        let mut step = c.len_utf8();
        match c {
            '"' => in_quote = !in_quote,
            _ if in_quote => {}
            '|' if in_pipe => in_pipe = false,
            _ if in_pipe => {}
            '@' if depth == 0 && s[i + 1..].starts_with('{') => return Some(i),
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => in_pipe = true,
            _ if depth == 0 => {
                if let Some(n) = crate::parser::edge_span(&s[i..]) {
                    step = n;
                }
            }
            _ => {}
        }
        i += step;
    }
    None
}

/// Whether `s` leaves a shape-data block open: some `@{` outside quotes
/// whose closing `}` is not on this text.
pub(crate) fn leaves_open(s: &str) -> bool {
    let mut rest = s;
    while let Some(o) = find_open(rest) {
        let body = &rest[o + 2..];
        match body_end(body) {
            Some(e) => rest = &body[e + 1..],
            None => return true,
        }
    }
    false
}

/// Mermaid's lexer rewrite of a raw body: inside a double-quoted string
/// every newline and the indentation after it become `<br/>`.
pub(crate) fn fold_quoted_newlines(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_quote = false;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            in_quote = !in_quote;
        }
        if in_quote && c == '\n' {
            out.push_str("<br/>");
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Read a shape-data body (the text between `@{` and `}`, after
/// [`fold_quoted_newlines`]) into its `(key, value)` entries, in order.
pub(crate) fn parse(body: &str) -> Result<Vec<(String, Yaml)>, String> {
    let entries = if body.contains('\n') {
        block_mapping(body)?
    } else {
        flow_mapping(body)?
    };
    let mut seen = std::collections::HashSet::new();
    for (k, _) in &entries {
        if !seen.insert(k.as_str()) {
            return Err(format!("duplicated mapping key '{k}' in '@{{…}}'"));
        }
    }
    Ok(entries)
}

/// A character cursor over one line of shape data.
struct Cur<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Cur<'a> {
    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn skip_blanks(&mut self) {
        let r = self.rest();
        self.pos += r.len() - r.trim_start_matches([' ', '\t']).len();
    }
    /// At end, or at a ` #` comment (only valid after whitespace or at
    /// the very start).
    fn at_end(&self) -> bool {
        let r = self.rest();
        r.is_empty() || (r.starts_with('#') && self.after_blank())
    }
    fn after_blank(&self) -> bool {
        self.pos == 0 || self.s[..self.pos].ends_with([' ', '\t'])
    }
}

/// A `:` that ends a key: followed by whitespace, the end, or (in flow
/// context) a flow indicator.
fn is_key_colon(r: &str, flow: bool) -> bool {
    let Some(after) = r.strip_prefix(':') else {
        return false;
    };
    match after.chars().next() {
        None => true,
        Some(c) => c == ' ' || c == '\t' || (flow && matches!(c, ',' | '}' | ']')),
    }
}

/// A quoted scalar at the cursor (`"…"` with YAML escapes, `'…'` with
/// `''`), or `None` when the cursor is not on a quote.
fn quoted(cur: &mut Cur<'_>) -> Result<Option<String>, String> {
    let q = match cur.peek() {
        Some(q @ ('"' | '\'')) => q,
        _ => return Ok(None),
    };
    cur.pos += 1;
    let mut out = String::new();
    loop {
        let Some(c) = cur.peek() else {
            return Err("unterminated quoted string in '@{…}'".to_string());
        };
        cur.pos += c.len_utf8();
        match c {
            '\'' if q == '\'' => {
                if cur.peek() == Some('\'') {
                    cur.pos += 1;
                    out.push('\'');
                } else {
                    return Ok(Some(out));
                }
            }
            '"' if q == '"' => return Ok(Some(out)),
            '\\' if q == '"' => {
                let Some(e) = cur.peek() else {
                    return Err("unterminated quoted string in '@{…}'".to_string());
                };
                cur.pos += e.len_utf8();
                out.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    other => other,
                });
            }
            c => out.push(c),
        }
    }
}

/// Skip a bracketed flow collection (`[…]` / `{…}`), quote-aware.
fn skip_collection(cur: &mut Cur<'_>) -> Result<(), String> {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    while let Some(c) = cur.peek() {
        cur.pos += c.len_utf8();
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '[' | '{') => depth += 1,
            (None, ']' | '}') => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            _ => {}
        }
    }
    Err("unclosed collection in '@{…}'".to_string())
}

/// Type a plain (unquoted) scalar per js-yaml's JSON schema.
fn plain(text: &str) -> Yaml {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Yaml::Null,
        "true" | "True" | "TRUE" => return Yaml::Bool(true),
        "false" | "False" | "FALSE" => return Yaml::Bool(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => return Yaml::Num(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => return Yaml::Num(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return Yaml::Num(f64::NAN),
        _ => {}
    }
    let (neg, digits) = match text.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let radix = |p: &str, r: u32| {
        digits
            .strip_prefix(p)
            .filter(|d| !d.is_empty())
            .and_then(|d| i64::from_str_radix(d, r).ok())
    };
    if let Some(v) = radix("0x", 16).or_else(|| radix("0o", 8)) {
        return Yaml::Num(if neg { -(v as f64) } else { v as f64 });
    }
    let numeric = !digits.is_empty()
        && digits.chars().any(|c| c.is_ascii_digit())
        && digits
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
        && digits
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '.');
    match (numeric, digits.parse::<f64>()) {
        (true, Ok(v)) => Yaml::Num(if neg { -v } else { v }),
        _ => Yaml::Str(text.to_string()),
    }
}

/// A key at the cursor: quoted, or plain up to its `:`.
fn key(cur: &mut Cur<'_>, flow: bool) -> Result<String, String> {
    if let Some(k) = quoted(cur)? {
        return Ok(k);
    }
    let start = cur.pos;
    while let Some(c) = cur.peek() {
        let r = cur.rest();
        if is_key_colon(r, flow) || (flow && matches!(c, ',' | '}' | ']')) {
            break;
        }
        if c == '#' && cur.after_blank() {
            break;
        }
        cur.pos += c.len_utf8();
    }
    Ok(cur.s[start..cur.pos].trim_end().to_string())
}

/// `{ k: v, k2: v2 }` (the braces are implied) on one line.
fn flow_mapping(body: &str) -> Result<Vec<(String, Yaml)>, String> {
    let mut cur = Cur { s: body, pos: 0 };
    let mut out = Vec::new();
    loop {
        cur.skip_blanks();
        if cur.at_end() {
            return Ok(out);
        }
        if cur.peek() == Some(',') {
            return Err("unexpected ',' in '@{…}'".to_string());
        }
        let k = key(&mut cur, true)?;
        cur.skip_blanks();
        let v = if is_key_colon(cur.rest(), true) {
            cur.pos += 1;
            cur.skip_blanks();
            flow_value(&mut cur)?
        } else {
            Yaml::Null
        };
        out.push((k, v));
        cur.skip_blanks();
        if cur.at_end() {
            return Ok(out);
        }
        if cur.peek() != Some(',') {
            return Err(
                "missed comma between entries in '@{…}' (write `key: value, key: value`)"
                    .to_string(),
            );
        }
        cur.pos += 1;
    }
}

/// A value in flow context: quoted, a skipped collection, or plain up
/// to the next `,` (a `: ` inside it means a missing comma).
fn flow_value(cur: &mut Cur<'_>) -> Result<Yaml, String> {
    if cur.at_end() || cur.peek() == Some(',') {
        return Ok(Yaml::Null);
    }
    if let Some(q) = quoted(cur)? {
        return Ok(Yaml::Str(q));
    }
    if matches!(cur.peek(), Some('[' | '{')) {
        skip_collection(cur)?;
        return Ok(Yaml::Collection);
    }
    let start = cur.pos;
    while let Some(c) = cur.peek() {
        if c == ',' || matches!(c, '}' | ']') || is_key_colon(cur.rest(), true) {
            break;
        }
        if c == '#' && cur.after_blank() {
            break;
        }
        cur.pos += c.len_utf8();
    }
    Ok(plain(cur.s[start..cur.pos].trim_end()))
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

/// A multi-line body: one `key: value` per line at a common indent,
/// with `|` / `>` block scalars and folded plain continuations.
fn block_mapping(body: &str) -> Result<Vec<(String, Yaml)>, String> {
    let lines: Vec<&str> = body.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let mut out = Vec::new();
    let mut base: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if is_blank_or_comment(line) {
            continue;
        }
        if line.trim_start_matches(' ').starts_with('\t') {
            return Err("tab indentation is not allowed in '@{…}'".to_string());
        }
        let ind = indent_of(line);
        let base = *base.get_or_insert(ind);
        if ind != base {
            return Err("bad indentation of a mapping entry in '@{…}'".to_string());
        }
        let mut cur = Cur { s: line, pos: ind };
        let k = key(&mut cur, false)?;
        if !is_key_colon(cur.rest(), false) {
            return Err(format!(
                "expected `key: value` in '@{{…}}', found: '{}'",
                line.trim()
            ));
        }
        cur.pos += 1;
        cur.skip_blanks();
        // Lines indented past the key belong to its value.
        let mut more = Vec::new();
        while i < lines.len() && (lines[i].trim().is_empty() || indent_of(lines[i]) > base) {
            more.push(lines[i]);
            i += 1;
        }
        while more.last().is_some_and(|l| l.trim().is_empty()) {
            more.pop();
            i -= 1;
        }
        let v = block_value(&mut cur, &more)?;
        out.push((k, v));
    }
    Ok(out)
}

fn block_value(cur: &mut Cur<'_>, more: &[&str]) -> Result<Yaml, String> {
    if let Some(ind @ ('|' | '>')) = cur.peek() {
        let chomp = cur.rest()[1..].trim();
        if !matches!(chomp, "" | "-" | "+") {
            return Err(format!("unsupported block scalar header '{ind}{chomp}'"));
        }
        return Ok(Yaml::Str(block_scalar(ind == '|', chomp, more)?));
    }
    let v = if cur.at_end() {
        Yaml::Null
    } else if let Some(q) = quoted(cur)? {
        cur.skip_blanks();
        if !cur.at_end() {
            return Err("unexpected text after a quoted value in '@{…}'".to_string());
        }
        Yaml::Str(q)
    } else if matches!(cur.peek(), Some('[' | '{')) {
        skip_collection(cur)?;
        Yaml::Collection
    } else {
        let start = cur.pos;
        while let Some(c) = cur.peek() {
            if is_key_colon(cur.rest(), false) {
                return Err("bad indentation of a mapping entry in '@{…}'".to_string());
            }
            if c == '#' && cur.after_blank() {
                break;
            }
            cur.pos += c.len_utf8();
        }
        let mut text = cur.s[start..cur.pos].trim_end().to_string();
        // A plain scalar folds its continuation lines with spaces.
        for l in more.iter().filter(|l| !is_blank_or_comment(l)) {
            let t = l.trim();
            if t.ends_with(':') || t.contains(": ") {
                return Err("bad indentation of a mapping entry in '@{…}'".to_string());
            }
            text.push(' ');
            text.push_str(l.trim());
        }
        return Ok(plain(&text));
    };
    if more.iter().any(|l| !is_blank_or_comment(l)) {
        return Err("nested mappings are not supported in '@{…}'".to_string());
    }
    Ok(v)
}

/// A `|` (literal) or `>` (folded) block scalar from its indented
/// lines, with YAML's clip / strip (`-`) / keep (`+`) chomping. A port
/// of js-yaml's `readBlockScalar`: the content indent is that of the
/// first non-blank line (a blank line of spaces deeper than it counts
/// too), a later line indented less ends the scalar — and since such a
/// line is still inside the key's value, anything but a comment there is
/// js-yaml's "bad indentation" error — and a folded scalar keeps a
/// more-indented line (one starting with a space or tab past the content
/// indent) on its own line instead of folding it.
fn block_scalar(literal: bool, chomp: &str, lines: &[&str]) -> Result<String, String> {
    let spaces = |l: &str| l.len() - l.trim_start_matches(' ').len();
    let mut ind: Option<usize> = None;
    let mut probe = 0;
    let mut out = String::new();
    let (mut empty, mut read, mut more_indented) = (0usize, false, false);
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let n = match ind {
            Some(t) => spaces(l).min(t),
            None => spaces(l),
        };
        let rest = &l[n..];
        if rest.is_empty() {
            if ind.is_none() {
                probe = probe.max(n);
            }
            empty += 1;
            i += 1;
            continue;
        }
        let t = *ind.get_or_insert(probe.max(n));
        if n < t {
            break;
        }
        let gaps = |extra: usize| "\n".repeat(empty + extra);
        if literal {
            out.push_str(&gaps(usize::from(read)));
        } else if rest.starts_with([' ', '\t']) {
            more_indented = true;
            out.push_str(&gaps(usize::from(read)));
        } else if more_indented {
            more_indented = false;
            out.push_str(&gaps(1));
        } else if empty == 0 {
            if read {
                out.push(' ');
            }
        } else {
            out.push_str(&gaps(0));
        }
        out.push_str(rest);
        (empty, read) = (0, true);
        i += 1;
    }
    if lines[i..].iter().any(|l| !is_blank_or_comment(l)) {
        return Err("bad indentation of a mapping entry in '@{…}'".to_string());
    }
    Ok(match chomp {
        "-" => out,
        // Keep: every trailing break. The body's trailing blank lines
        // are already trimmed, so this counts the one break mermaid's
        // appended newline always leaves (js-yaml gives `|+` alone "\n").
        "+" => out + &"\n".repeat(empty + 1),
        _ if read => out + "\n",
        _ => out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(body: &str) -> Vec<(String, Yaml)> {
        parse(body).unwrap_or_else(|e| panic!("{e}: {body:?}"))
    }

    fn s(v: &str) -> Yaml {
        Yaml::Str(v.to_string())
    }

    #[test]
    fn flow_form_matches_js_yaml() {
        assert_eq!(
            kv(" shape: cyl, label: \"DB\" "),
            [("shape".into(), s("cyl")), ("label".into(), s("DB"))]
        );
        assert_eq!(kv("shape:cyl"), [("shape:cyl".into(), Yaml::Null)]);
        assert_eq!(
            kv(" label: Hello, world "),
            [("label".into(), s("Hello")), ("world".into(), Yaml::Null)]
        );
        assert_eq!(kv(" label: 'single' "), [("label".into(), s("single"))]);
        assert_eq!(kv(" a: 'it''s' "), [("a".into(), s("it's"))]);
        assert_eq!(kv(" a: \"t\\tb\" "), [("a".into(), s("t\tb"))]);
        assert_eq!(kv(" \"a b\": 1 "), [("a b".into(), Yaml::Num(1.0))]);
        assert_eq!(kv(" label: x # c "), [("label".into(), s("x"))]);
        assert_eq!(kv(" label: x#c "), [("label".into(), s("x#c"))]);
        assert_eq!(kv(" a: "), [("a".into(), Yaml::Null)]);
        assert_eq!(kv(" a "), [("a".into(), Yaml::Null)]);
        assert_eq!(kv(""), []);
        assert_eq!(kv(" label: [1,2] "), [("label".into(), Yaml::Collection)]);
        for bad in [
            " shape: cyl label: x ",
            " label: a: b ",
            " a: \"x\" y ",
            " shape: cyl, shape: rect ",
            " a: \"open ",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn json_schema_typing() {
        let got = kv(" a: yes, b: Null, c: ~, d: TRUE, e: 0x1F, f: 1e3, g: .5, h: 01, i: +5, j: 1_000, k: false, l: 0");
        let want = [
            s("yes"),
            Yaml::Null,
            Yaml::Null,
            Yaml::Bool(true),
            Yaml::Num(31.0),
            Yaml::Num(1000.0),
            Yaml::Num(0.5),
            Yaml::Num(1.0),
            Yaml::Num(5.0),
            s("1_000"),
            Yaml::Bool(false),
            Yaml::Num(0.0),
        ];
        let vals: Vec<Yaml> = got.into_iter().map(|(_, v)| v).collect();
        assert_eq!(vals, want);
        assert!(s("yes").truthy() && s("false").truthy());
        assert!(!Yaml::Num(0.0).truthy() && !Yaml::Null.truthy() && !s("").truthy());
        assert_eq!(Yaml::Num(123.0).text().as_deref(), Some("123"));
        assert_eq!(Yaml::Num(1.5).text().as_deref(), Some("1.5"));
        assert_eq!(Yaml::Bool(true).text().as_deref(), Some("true"));
    }

    #[test]
    fn block_form_matches_js_yaml() {
        assert_eq!(
            kv("\n  shape: cyl,\n  label: DB\n"),
            [("shape".into(), s("cyl,")), ("label".into(), s("DB"))]
        );
        assert_eq!(
            kv("\n  label: |\n    line a\n    line b\n  other: \"x\"\n"),
            [
                ("label".into(), s("line a\nline b\n")),
                ("other".into(), s("x"))
            ]
        );
        assert_eq!(
            kv("\n  label: >\n    line a\n    line b\n"),
            [("label".into(), s("line a line b\n"))]
        );
        assert_eq!(kv("\n label: |-\n   x\n\n"), [("label".into(), s("x"))]);
        assert_eq!(
            kv("\n label: |\n   x\n\n   y\n"),
            [("label".into(), s("x\n\ny\n"))]
        );
        assert_eq!(
            kv("\n label: >\n   a\n   b\n\n   c\n"),
            [("label".into(), s("a b\nc\n"))]
        );
        // js-yaml's readBlockScalar: more-indented lines are not
        // folded; the content indent comes from the first line.
        assert_eq!(
            kv("\n  label: >\n    a\n      b\n    c\n"),
            [("label".into(), s("a\n  b\nc\n"))]
        );
        assert_eq!(
            kv("\n  label: >\n    a\n\n      b\n    c\n    d\n"),
            [("label".into(), s("a\n\n  b\nc d\n"))]
        );
        assert_eq!(
            kv("\n  label: |\n      a\n    # c\n"),
            [("label".into(), s("a\n"))]
        );
        assert_eq!(kv("\n  label: |+\n"), [("label".into(), s("\n"))]);
        for bad in [
            "\n  label: |\n      a\n    b\n",
            "\n  label: >\n      a\n    b\n",
            "\n  label: |\n\n        \n      a\n",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        assert_eq!(
            kv("\n label: This is\n   continued\n"),
            [("label".into(), s("This is continued"))]
        );
        assert_eq!(
            kv("\n a:\n b: 2\n"),
            [("a".into(), Yaml::Null), ("b".into(), Yaml::Num(2.0))]
        );
        assert_eq!(
            kv("\n  # a comment\n  shape: cyl\n"),
            [("shape".into(), s("cyl"))]
        );
        for bad in [
            "\n  shape: cyl, label: DB\n",
            "\n a: 1\n   b: 2\n",
            "\n\ta: 1\n",
            "\n a: 1\n  b: 2\n",
            "\n  a: 1\n b: 2\n",
            "\n  just text\n",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn block_end_and_open_follow_the_mermaid_lexer() {
        assert_eq!(body_end(" label: \"a}b\" }"), Some(14));
        assert_eq!(body_end(" label: 'a}b' }"), Some(10));
        assert_eq!(body_end(" shape: cyl"), None);
        assert_eq!(find_open("X[\"a @{ b }\"]"), None);
        assert_eq!(find_open("A@{ shape: cyl }"), Some(1));
        assert!(leaves_open("A --> D@{"));
        assert!(!leaves_open("A@{ x } --> B@{ y }"));
        assert!(leaves_open("A@{ x } --> B@{ y"));
        // `@{` in label text is text, as in mermaid's text states.
        for s in [
            "A[x @{ y] --> B",
            "A(x @{ y) --> B",
            "A{x @{ y} --> B",
            "A -->|x @{ y| B",
            "A -- x @{ y --> B",
            "A -- x @{ y",
            "A --- B",
        ] {
            assert_eq!(find_open(s), None, "{s}");
            assert!(!leaves_open(s), "{s}");
        }
        assert_eq!(find_open("A -- t --> B@{"), Some(12));
        assert_eq!(find_open("A[x]@{ shape: cyl }"), Some(4));
        assert_eq!(
            fold_quoted_newlines(" label: \"This is a\n      multiline string\"\n"),
            " label: \"This is a<br/>multiline string\"\n"
        );
    }
}
