// ------------------------------------------------------------------
// Theme directives
// ------------------------------------------------------------------

/// The colour theme a diagram's source asks for (report M10, P1 "theme
/// from init/frontmatter ignored"): `theme:` and `themeVariables:`
/// under its YAML frontmatter's `config:`, then every `%%{init: …}%%`
/// (or `%%{initialize: …}%%`) directive in source order, a later one
/// winning — mermaid lays directives over the frontmatter config. A
/// theme name [`Theme::from_name`] does not know is the default theme,
/// `Some(Theme::Flowmaid)`, as mermaid falls back to its default; the
/// theme is `None` when nothing names one. Never fails: a directive
/// that does not parse is skipped, as the diagram parsers skip it.
pub(crate) fn theme_directive(source: &str) -> (Option<Theme>, ThemeVars) {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let named = |n: &str| Some(Theme::from_name(n).unwrap_or_default());
    let mut theme = frontmatter_config_value(source, "theme").and_then(|v| named(&unquote_yaml(v)));
    let mut vars = ThemeVars::default();
    // Only `themeVariables`' direct children: a nested namespace
    // (`themeVariables.sankey`) holds no theme colour.
    let lines = frontmatter_section(source, "themeVariables");
    if let Some(direct) = lines.iter().map(|(ind, _)| *ind).min() {
        for (_, t) in lines.iter().filter(|(ind, _)| *ind == direct) {
            if let Some((k, v)) = yaml_pair(t) {
                vars.set(k, &unquote_yaml(v));
            }
        }
    }
    for body in init_directives(source) {
        if let Some(Json::Str(name)) = json_get(&body, "theme") {
            theme = named(name);
        }
        if let Some(Json::Obj(entries)) = json_get(&body, "themeVariables") {
            for (k, v) in entries {
                if let Json::Str(v) = v {
                    vars.set(k, v);
                }
            }
        }
    }
    (theme, vars)
}

/// The value of a scalar `config.<key>: value` line in YAML frontmatter
/// — a DIRECT child of `config:` (raw: run it through
/// [`unquote_yaml`]), `None` when there is none or it opens a block.
fn frontmatter_config_value<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let body = frontmatter_body(source)?;
    let indent_of = |l: &str| l.len() - l.trim_start().len();
    let mut config_at: Option<usize> = None;
    let mut child_at: Option<usize> = None;
    let mut found = None;
    for line in body {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let ind = indent_of(line);
        if let Some(outer) = config_at {
            if ind > outer {
                let direct = *child_at.get_or_insert(ind);
                if ind == direct {
                    if let Some((k, v)) = yaml_pair(t) {
                        if k == key && !v.trim().is_empty() {
                            found = Some(v);
                        }
                    }
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
    found
}

/// The configuration object of every `%%{init: {…}}%%` /
/// `%%{initialize: {…}}%%` directive in `source`, in order. A
/// directive may span lines (it runs to its `}%%`); other directive
/// kinds (`%%{wrap}%%`) and bodies that do not parse are skipped.
fn init_directives(source: &str) -> Vec<Vec<(String, Json)>> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(open) = rest.find("%%{") {
        let after = &rest[open + 3..];
        let Some(close) = after.find("}%%") else {
            break;
        };
        let inner = after[..close].trim_start();
        rest = &after[close + 3..];
        let name_len = inner
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(inner.len());
        if !matches!(&inner[..name_len], "init" | "initialize") {
            continue;
        }
        let Some(value) = inner[name_len..].trim_start().strip_prefix(':') else {
            continue;
        };
        let mut at = 0;
        if let Some(Json::Obj(body)) = json_value(value, &mut at) {
            out.push(body);
        }
    }
    out
}

/// A value in a directive's JSON body, read leniently as mermaid's own
/// directive parser does: single or double quotes, bare keys, and bare
/// words as strings (`theme: dark`). Arrays are skipped whole.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Str(String),
    Obj(Vec<(String, Json)>),
    Other,
}

/// The value of `key` in a parsed object (the last one, as in JSON).
fn json_get<'a>(obj: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    obj.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn json_skip_ws(s: &str, at: &mut usize) {
    let b = s.as_bytes();
    while b.get(*at).is_some_and(|c| c.is_ascii_whitespace()) {
        *at += 1;
    }
}

/// One JSON value starting at byte `*at` (whitespace first), leaving
/// `*at` just past it; `None` where it is not one.
fn json_value(s: &str, at: &mut usize) -> Option<Json> {
    json_skip_ws(s, at);
    let b = s.as_bytes();
    match *b.get(*at)? {
        b'{' => {
            *at += 1;
            let mut out = Vec::new();
            loop {
                json_skip_ws(s, at);
                if b.get(*at) == Some(&b'}') {
                    *at += 1;
                    return Some(Json::Obj(out));
                }
                let key = match *b.get(*at)? {
                    b'"' | b'\'' => json_string(s, at)?,
                    _ => json_bare(s, at)?,
                };
                json_skip_ws(s, at);
                if b.get(*at) != Some(&b':') {
                    return None;
                }
                *at += 1;
                let v = json_value(s, at)?;
                out.push((key, v));
                json_skip_ws(s, at);
                match b.get(*at)? {
                    b',' => *at += 1,
                    b'}' => {}
                    _ => return None,
                }
            }
        }
        b'[' => {
            // Skip to the matching `]`, minding strings.
            let mut depth = 0usize;
            while let Some(&c) = b.get(*at) {
                match c {
                    b'"' | b'\'' => {
                        json_string(s, at)?;
                        continue;
                    }
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            *at += 1;
                            return Some(Json::Other);
                        }
                    }
                    _ => {}
                }
                *at += 1;
            }
            None
        }
        b'"' | b'\'' => json_string(s, at).map(Json::Str),
        _ => json_bare(s, at).map(Json::Str),
    }
}

/// A quoted string at `*at` (either quote), backslash escapes taken
/// literally (`\"` is `"`).
fn json_string(s: &str, at: &mut usize) -> Option<String> {
    let q = s[*at..].chars().next()?;
    let mut out = String::new();
    let mut chars = s[*at + 1..].char_indices();
    while let Some((i, c)) = chars.next() {
        if c == q {
            *at += 1 + i + 1;
            return Some(out);
        }
        if c == '\\' {
            if let Some((_, e)) = chars.next() {
                out.push(e);
            }
            continue;
        }
        out.push(c);
    }
    None
}

/// A bare word at `*at` (a key or an unquoted value): everything up to
/// whitespace or JSON punctuation; `None` when empty.
fn json_bare(s: &str, at: &mut usize) -> Option<String> {
    let rest = &s[*at..];
    let len = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | ':' | '{' | '}' | '[' | ']'))
        .unwrap_or(rest.len());
    if len == 0 {
        return None;
    }
    *at += len;
    Some(rest[..len].to_string())
}

