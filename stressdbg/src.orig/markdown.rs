//! Mermaid markdown strings in flowchart labels (`A["`**bold** _it_`"]`).
//!
//! Mermaid lexes `"` + backtick … backtick + `"` as a markdown string and
//! renders it with `marked` (flowchart default: HTML labels), keeping only
//! paragraphs, `**strong**` / `__strong__`, `*em*` / `_em_`, inline HTML
//! and backslash escapes; every other markdown construct (headings,
//! lists, links, code, strikethrough) is printed as its raw source. A
//! newline is a line break (its indentation dropped), blank lines are one
//! break, and long node labels wrap.
//!
//! flowmaid lowers the string at parse time into its own label markup —
//! `<b>…</b>`, `<i>…</i>` and `\n` — which the existing measuring and
//! rendering already understand, so no model change is needed and
//! labels without markdown are untouched.

use crate::layout::{text_width, TEXT_CALIBRATION};
use crate::parser::{decode_entities, normalize_breaks};

/// Opening and closing delimiters of a mermaid markdown string.
pub(crate) const MD_OPEN: &str = "\"`";
pub(crate) const MD_CLOSE: &str = "`\"";

/// Font size (px) mermaid lays labels out at.
const MERMAID_LABEL_FONT_PX: f64 = 16.0;
/// Mermaid's markdown wrapping width, in its px: `flowchart.wrappingWidth`
/// in mermaid 11 (also the fixed width of edge labels). Mermaid 12
/// lowered the node default to 120.
const MD_WRAP_WIDTH_MERMAID_PX: f64 = 200.0;
/// The same width in flowmaid's units (14px text), so the same words
/// break as in mermaid's 16px rendering.
const MD_WRAP_WIDTH: f64 = MD_WRAP_WIDTH_MERMAID_PX * TEXT_CALIBRATION / MERMAID_LABEL_FONT_PX;

/// The inner text of a whole-label markdown string (`"`…`"`), if `s`
/// (trimmed) is one.
pub(crate) fn md_quoted(s: &str) -> Option<&str> {
    let t = s.trim();
    (t.len() >= MD_OPEN.len() + MD_CLOSE.len() && t.starts_with(MD_OPEN) && t.ends_with(MD_CLOSE))
        .then(|| &t[MD_OPEN.len()..t.len() - MD_CLOSE.len()])
}

/// Whether `s` ends inside a markdown string: one was opened and its
/// closer has not come yet. A plain `"…"` string protects its contents
/// (it cannot open one).
pub(crate) fn leaves_open(s: &str) -> bool {
    #[derive(PartialEq)]
    enum St {
        Top,
        Quote,
        Md,
    }
    let mut st = St::Top;
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        let mut step = c.len_utf8();
        match st {
            St::Top if rest.starts_with(MD_OPEN) => {
                st = St::Md;
                step = MD_OPEN.len();
            }
            St::Top if c == '"' => st = St::Quote,
            St::Quote if c == '"' => st = St::Top,
            St::Md if rest.starts_with(MD_CLOSE) => {
                st = St::Top;
                step = MD_CLOSE.len();
            }
            _ => {}
        }
        rest = &rest[step..];
    }
    st == St::Md
}

/// Lower the body of a markdown string into label markup. `wrap`
/// greedy-wraps each line at mermaid's width (node labels);
/// `single_line` joins the lines with spaces (edge labels and subgraph
/// titles, which are one line in flowmaid). A raw backtick or `"` in
/// the body is an error, as in mermaid's lexer.
pub(crate) fn lower(raw: &str, wrap: bool, single_line: bool) -> Result<String, String> {
    if raw.contains(['`', '"']) {
        return Err("a markdown string cannot contain a raw backtick or quote \
                    — write #96; or #quot;"
            .to_string());
    }
    Ok(lower_text(raw, wrap, single_line))
}

/// [`lower`] for markdown that never went through mermaid's string
/// lexer (a node-metadata `label`), so backticks and quotes are plain
/// characters, printed as written like any other unsupported markdown.
pub(crate) fn lower_text(raw: &str, wrap: bool, single_line: bool) -> String {
    let mut lines: Vec<Line> = Vec::new();
    for unit in units(&normalize_breaks(raw)) {
        match unit {
            Unit::Literal(t) => lines.push(vec![(decode_entities(&t), false, false)]),
            Unit::Para(t) => lines.extend(inline(&t)),
        }
    }
    let lines: Vec<Line> = lines.into_iter().map(trim_line).collect();
    let out: Vec<String> = if wrap {
        lines.into_iter().flat_map(wrap_line).collect()
    } else {
        lines.iter().map(markup).collect()
    };
    out.join(if single_line { " " } else { "\n" })
}

/// Styled runs of one output line: (text, bold, italic).
type Line = Vec<(String, bool, bool)>;

/// A block of the body: a paragraph (consecutive lines, emphasis may
/// span them) or a line that `marked` would make a heading, list item,
/// quote or rule — mermaid prints those raw, so they stay literal.
enum Unit {
    Para(String),
    Literal(String),
}

fn units(body: &str) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let flush = |para: &mut Vec<&str>, out: &mut Vec<Unit>| {
        if !para.is_empty() {
            out.push(Unit::Para(para.join("\n")));
            para.clear();
        }
    };
    for line in body.split('\n').map(str::trim) {
        if line.is_empty() {
            flush(&mut para, &mut out);
        } else if is_block_line(line) {
            flush(&mut para, &mut out);
            out.push(Unit::Literal(line.to_string()));
        } else {
            para.push(line);
        }
    }
    flush(&mut para, &mut out);
    out
}

/// A line `marked` lexes as a block other than a paragraph: an ATX
/// heading, a bullet or ordered list item, a block quote, or a
/// thematic break.
fn is_block_line(l: &str) -> bool {
    let marker_then_space = |rest: &str| rest.is_empty() || rest.starts_with([' ', '\t']);
    let hashes = l.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) && marker_then_space(&l[hashes..]) {
        return true;
    }
    if l.starts_with(['-', '*', '+']) && marker_then_space(&l[1..]) {
        return true;
    }
    let digits = l.chars().take_while(char::is_ascii_digit).count();
    if (1..=9).contains(&digits)
        && l[digits..].starts_with(['.', ')'])
        && marker_then_space(&l[digits + 1..])
    {
        return true;
    }
    if l.starts_with('>') {
        return true;
    }
    let squashed: String = l.chars().filter(|c| !c.is_whitespace()).collect();
    squashed.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|&m| squashed.chars().all(|c| c == m))
}

/// One emphasis level a matched delimiter pair adds.
#[derive(Clone, Copy)]
enum Sty {
    Strong,
    Em,
}

enum Tok {
    Text(String),
    Delim {
        ch: char,
        /// Run length as written (for the rule of three).
        orig: usize,
        /// Characters not consumed by a match (printed literally).
        rem: usize,
        can_open: bool,
        can_close: bool,
        opens: Vec<Sty>,
        closes: Vec<Sty>,
    },
}

/// Punctuation in CommonMark's flanking sense (approximated as any
/// visible non-alphanumeric character).
fn is_punct(c: char) -> bool {
    !c.is_alphanumeric() && !c.is_whitespace()
}

/// Inline-parse one paragraph (CommonMark's delimiter-run algorithm,
/// restricted to `*` and `_`) into styled lines.
fn inline(para: &str) -> Vec<Line> {
    let mut toks = tokenize(para);
    match_emphasis(&mut toks);
    serialise(&toks)
}

/// Split a paragraph into text and `*` / `_` delimiter runs, with
/// CommonMark's flanking rules deciding which runs may open or close.
fn tokenize(para: &str) -> Vec<Tok> {
    let chars: Vec<char> = para.chars().collect();
    let mut toks: Vec<Tok> = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && chars.get(i + 1).is_some_and(|n| n.is_ascii_punctuation()) {
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '*' || c == '_' {
            let n = chars[i..].iter().take_while(|&&d| d == c).count();
            let prev = if i == 0 { None } else { Some(chars[i - 1]) };
            let next = chars.get(i + n).copied();
            let ws = |o: Option<char>| o.map_or(true, char::is_whitespace);
            let pu = |o: Option<char>| o.is_some_and(is_punct);
            let left = !ws(next) && (!pu(next) || ws(prev) || pu(prev));
            let right = !ws(prev) && (!pu(prev) || ws(next) || pu(next));
            let (can_open, can_close) = if c == '*' {
                (left, right)
            } else {
                (left && (!right || pu(prev)), right && (!left || pu(next)))
            };
            if !text.is_empty() {
                toks.push(Tok::Text(std::mem::take(&mut text)));
            }
            toks.push(Tok::Delim {
                ch: c,
                orig: n,
                rem: n,
                can_open,
                can_close,
                opens: Vec::new(),
                closes: Vec::new(),
            });
            i += n;
            continue;
        }
        text.push(c);
        i += 1;
    }
    if !text.is_empty() {
        toks.push(Tok::Text(text));
    }
    toks
}

/// End of a delimiter-list link (no neighbour).
const NIL: usize = usize::MAX;

/// CommonMark "process emphasis": pair each closer with the nearest
/// usable opener of the same character, two characters at a time when
/// both runs allow it (strong), else one (em).
///
/// Linear-ish like the reference algorithm: the delimiters form a
/// doubly linked list that matched and spent runs are unlinked from
/// (each at most once), and a failed search records an "openers
/// bottom" per closer kind (character, can-open, run length mod 3 —
/// everything the opener test depends on), so no later closer of that
/// kind rescans the same stretch. A pathological label like `_*_*…`
/// is then linear instead of quadratic.
fn match_emphasis(toks: &mut [Tok]) {
    let delims: Vec<usize> = (0..toks.len())
        .filter(|&i| matches!(toks[i], Tok::Delim { .. }))
        .collect();
    let n = delims.len();
    let mut prev: Vec<usize> = (0..n).map(|k| k.checked_sub(1).unwrap_or(NIL)).collect();
    let mut next: Vec<usize> = (0..n)
        .map(|k| if k + 1 < n { k + 1 } else { NIL })
        .collect();
    let unlink = |k: usize, prev: &mut [usize], next: &mut [usize]| {
        let (p, q) = (prev[k], next[k]);
        if p != NIL {
            next[p] = q;
        }
        if q != NIL {
            prev[q] = p;
        }
    };
    // Lowest delimiter (list index) an opener may sit at, per closer
    // kind: [is `_`][closer can open][closer run length % 3].
    let mut bottom = [[[0usize; 3]; 2]; 2];
    let mut c = if n > 0 { 0 } else { NIL };
    while c != NIL {
        let (cch, corig, crem, ccan_open, ccan_close) = match &toks[delims[c]] {
            Tok::Delim {
                ch,
                orig,
                rem,
                can_open,
                can_close,
                ..
            } => (*ch, *orig, *rem, *can_open, *can_close),
            Tok::Text(_) => unreachable!("delimiter list holds delimiters"),
        };
        if !ccan_close || crem == 0 {
            let nc = next[c];
            if !ccan_open || crem == 0 {
                unlink(c, &mut prev, &mut next);
            }
            c = nc;
            continue;
        }
        let floor = &mut bottom[usize::from(cch == '_')][usize::from(ccan_open)][corig % 3];
        let mut o = prev[c];
        let opener = loop {
            if o == NIL || o < *floor {
                break None;
            }
            if let Tok::Delim {
                ch,
                orig,
                rem,
                can_open: true,
                can_close,
                ..
            } = &toks[delims[o]]
            {
                let odd = (*can_close || ccan_open)
                    && (orig + corig) % 3 == 0
                    && !(orig % 3 == 0 && corig % 3 == 0);
                if *ch == cch && *rem > 0 && !odd {
                    break Some(o);
                }
            }
            o = prev[o];
        };
        let Some(o) = opener else {
            // Nothing below here can open this kind of closer.
            *floor = c;
            let nc = next[c];
            if !ccan_open {
                unlink(c, &mut prev, &mut next);
            }
            c = nc;
            continue;
        };
        let orem = match &toks[delims[o]] {
            Tok::Delim { rem, .. } => *rem,
            Tok::Text(_) => unreachable!("opener is a delimiter"),
        };
        let used = if orem >= 2 && crem >= 2 { 2 } else { 1 };
        let sty = if used == 2 { Sty::Strong } else { Sty::Em };
        if let Tok::Delim { rem, opens, .. } = &mut toks[delims[o]] {
            *rem -= used;
            opens.push(sty);
        }
        if let Tok::Delim { rem, closes, .. } = &mut toks[delims[c]] {
            *rem -= used;
            closes.push(sty);
        }
        // Delimiters between the pair can no longer match.
        let mut k = next[o];
        while k != c {
            let nk = next[k];
            unlink(k, &mut prev, &mut next);
            k = nk;
        }
        if orem == used {
            unlink(o, &mut prev, &mut next);
        }
        if crem == used {
            let nc = next[c];
            unlink(c, &mut prev, &mut next);
            c = nc;
        }
    }
}

/// Walk the matched tokens into styled lines: a delimiter prints its
/// closes, then its unconsumed characters, then its opens (outermost
/// first); text is entity-decoded here, after emphasis, so `#42;` is
/// never a delimiter.
fn serialise(toks: &[Tok]) -> Vec<Line> {
    let mut lines: Vec<Line> = vec![Vec::new()];
    let (mut bold, mut ital) = (0usize, 0usize);
    let push = |lines: &mut Vec<Line>, t: &str, b: usize, i: usize| {
        for (k, part) in t.split('\n').enumerate() {
            if k > 0 {
                lines.push(Vec::new());
            }
            if !part.is_empty() {
                let line = lines.last_mut().expect("non-empty");
                line.push((part.to_string(), b > 0, i > 0));
            }
        }
    };
    let bump = |s: Sty, b: &mut usize, i: &mut usize, up: bool| {
        let n = match s {
            Sty::Strong => b,
            Sty::Em => i,
        };
        *n = if up { *n + 1 } else { n.saturating_sub(1) };
    };
    for t in toks {
        match t {
            Tok::Text(s) => push(&mut lines, &decode_entities(s), bold, ital),
            Tok::Delim {
                ch,
                rem,
                opens,
                closes,
                ..
            } => {
                for &s in closes {
                    bump(s, &mut bold, &mut ital, false);
                }
                push(&mut lines, &ch.to_string().repeat(*rem), bold, ital);
                for &s in opens.iter().rev() {
                    bump(s, &mut bold, &mut ital, true);
                }
            }
        }
    }
    lines
}

/// Drop leading and trailing whitespace of a styled line.
fn trim_line(mut l: Line) -> Line {
    if let Some(first) = l.first_mut() {
        first.0 = first.0.trim_start().to_string();
    }
    if let Some(last) = l.last_mut() {
        last.0 = last.0.trim_end().to_string();
    }
    l.retain(|r| !r.0.is_empty());
    l
}

/// Label markup for a styled line: adjacent runs of one style merge,
/// a styled run becomes `<b>…</b>`, `<i>…</i>` or `<b><i>…</i></b>`.
fn markup(l: &Line) -> String {
    let mut merged: Line = Vec::new();
    for (t, b, i) in l {
        match merged.last_mut() {
            Some(last) if last.1 == *b && last.2 == *i => last.0.push_str(t),
            _ => merged.push((t.clone(), *b, *i)),
        }
    }
    merged
        .into_iter()
        .map(|(t, b, i)| match (b, i) {
            (false, false) => t,
            (true, false) => format!("<b>{t}</b>"),
            (false, true) => format!("<i>{t}</i>"),
            (true, true) => format!("<b><i>{t}</i></b>"),
        })
        .collect()
}

/// Greedy word wrap at [`MD_WRAP_WIDTH`], measured with the same
/// estimate node sizing uses (bold counts wider, tags count zero). A
/// word wider than the limit keeps a line to itself, unsplit, as in
/// mermaid's HTML labels. A space keeps the style it was written in.
fn wrap_line(l: Line) -> Vec<String> {
    // Words (a word may mix styles), each with the style of the space
    // before it.
    let mut words: Vec<(Line, (bool, bool))> = vec![(Vec::new(), (false, false))];
    for (t, b, i) in l {
        for (k, part) in t.split(' ').enumerate() {
            if k > 0 {
                if words.last().is_some_and(|w| w.0.is_empty()) {
                    words.pop();
                }
                words.push((Vec::new(), (b, i)));
            }
            if !part.is_empty() {
                let w = &mut words.last_mut().expect("non-empty").0;
                w.push((part.to_string(), b, i));
            }
        }
    }
    words.retain(|w| !w.0.is_empty());
    let mut out: Vec<String> = Vec::new();
    let mut cur: Line = Vec::new();
    for (w, (sb, si)) in words {
        if cur.is_empty() {
            cur = w;
            continue;
        }
        let mut cand = cur.clone();
        cand.push((" ".to_string(), sb, si));
        cand.extend(w.iter().cloned());
        if text_width(&markup(&cand)) > MD_WRAP_WIDTH {
            out.push(markup(&cur));
            cur = w;
        } else {
            cur = cand;
        }
    }
    out.push(markup(&cur));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lo(s: &str) -> String {
        lower(s, false, false).unwrap()
    }

    /// The pre-optimisation quadratic matcher, kept as the reference
    /// the linked-list version must agree with.
    fn naive_match_emphasis(toks: &mut [Tok]) {
        for c in 0..toks.len() {
            loop {
                let (cch, corig, crem, ccan_open) = match &toks[c] {
                    Tok::Delim {
                        ch,
                        orig,
                        rem,
                        can_close: true,
                        can_open,
                        ..
                    } if *rem > 0 => (*ch, *orig, *rem, *can_open),
                    _ => break,
                };
                let opener = (0..c).rev().find(|&o| match &toks[o] {
                    Tok::Delim {
                        ch,
                        orig,
                        rem,
                        can_open: true,
                        can_close,
                        ..
                    } if *ch == cch && *rem > 0 => {
                        let odd = (*can_close || ccan_open)
                            && (orig + corig) % 3 == 0
                            && !(orig % 3 == 0 && corig % 3 == 0);
                        !odd
                    }
                    _ => false,
                });
                let Some(o) = opener else { break };
                let orem = match &toks[o] {
                    Tok::Delim { rem, .. } => *rem,
                    Tok::Text(_) => unreachable!("opener is a delimiter"),
                };
                let used = if orem >= 2 && crem >= 2 { 2 } else { 1 };
                let sty = if used == 2 { Sty::Strong } else { Sty::Em };
                if let Tok::Delim { rem, opens, .. } = &mut toks[o] {
                    *rem -= used;
                    opens.push(sty);
                }
                if let Tok::Delim { rem, closes, .. } = &mut toks[c] {
                    *rem -= used;
                    closes.push(sty);
                }
                // Delimiters between the pair can no longer match.
                for t in &mut toks[o + 1..c] {
                    if let Tok::Delim {
                        can_open,
                        can_close,
                        ..
                    } = t
                    {
                        *can_open = false;
                        *can_close = false;
                    }
                }
            }
        }
    }

    #[test]
    fn linear_matcher_agrees_with_the_reference() {
        // Deterministic LCG over a delimiter-heavy alphabet.
        const ALPHABET: &[char] = &['*', '_', 'a', ' ', '!', '*', '_', '\n'];
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        for _ in 0..4000 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let len = (seed >> 59) as usize + 1;
            let para: String = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    ALPHABET[(seed >> 61) as usize]
                })
                .collect();
            let mut want = tokenize(&para);
            naive_match_emphasis(&mut want);
            assert_eq!(inline(&para), serialise(&want), "{para:?}");
        }
    }

    #[test]
    fn pathological_runs_stay_linear() {
        // 256 KB of alternating, never-matching delimiters used to take
        // seconds (every closer rescanned every earlier run).
        const LABEL_BYTES: usize = 256 * 1024;
        let bound = if cfg!(debug_assertions) {
            std::time::Duration::from_secs(5)
        } else {
            std::time::Duration::from_millis(500)
        };
        for unit in ["_*", "*_", "**_", "_ *", "a_*b"] {
            let label = unit.repeat(LABEL_BYTES / unit.len());
            let t = std::time::Instant::now();
            lower(&label, false, false).unwrap();
            let took = t.elapsed();
            assert!(took < bound, "{unit:?}: {took:?}");
        }
    }

    #[test]
    fn emphasis_follows_commonmark() {
        assert_eq!(
            lo("This **is** _Markdown_"),
            "This <b>is</b> <i>Markdown</i>"
        );
        assert_eq!(
            lo("***both*** __strong__ *em* _em2_"),
            "<b><i>both</i></b> <b>strong</b> <i>em</i> <i>em2</i>"
        );
        // No emphasis: intraword `_`, spaced `*`, unclosed runs.
        let s = "snake_case_name and 2 * 3 * 4 and **unclosed";
        assert_eq!(lo(s), s);
        // Intraword `*` is emphasis.
        assert_eq!(lo("a*b*c"), "a<i>b</i>c");
        assert_eq!(lo("**a *b* c**"), "<b>a </b><b><i>b</i></b><b> c</b>");
        assert_eq!(lo("*a **b** c*"), "<i>a </i><b><i>b</i></b><i> c</i>");
        // Rule of three.
        assert_eq!(
            lo("*foo**bar**baz*"),
            "<i>foo</i><b><i>bar</i></b><i>baz</i>"
        );
        // …unless both runs are multiples of three.
        assert_eq!(lo("foo***bar***baz"), "foo<b><i>bar</i></b>baz");
    }

    #[test]
    fn escapes_and_entities_stay_literal() {
        assert_eq!(
            lo("\\*not italic\\* #42;also not#42;"),
            "*not italic* *also not*"
        );
        assert_eq!(lo("#95;x#95;"), "_x_");
        assert_eq!(lo("a\\b"), "a\\b");
    }

    #[test]
    fn lines_blocks_and_breaks() {
        assert_eq!(lo("Line1\n    Line 2\n    Line 3"), "Line1\nLine 2\nLine 3");
        assert_eq!(lo("a\n\n\n  b"), "a\nb");
        assert_eq!(lo("one<br/>two **b**"), "one\ntwo <b>b</b>");
        assert_eq!(lo("# heading\n  - item"), "# heading\n- item");
        assert_eq!(lo("1. **x**\n2) y\n> q\n---"), "1. **x**\n2) y\n> q\n---");
        // Emphasis spans a soft line break.
        assert_eq!(lo("**a\nb**"), "<b>a</b>\n<b>b</b>");
        assert_eq!(lower("two\n  lines", false, true).unwrap(), "two lines");
        // Links and strikethrough print raw, as in mermaid's HTML path.
        assert_eq!(lo("[a](b) ~~x~~"), "[a](b) ~~x~~");
    }

    #[test]
    fn raw_backticks_and_quotes_are_errors() {
        assert!(lower("bad ` tick", false, false).is_err());
        assert!(lower("say \"hi\"", false, false).is_err());
        assert_eq!(lo(""), "");
    }

    #[test]
    fn wrapping_is_greedy_and_never_splits_words() {
        let out = lower(
            "The quick brown fox jumps over the lazy dog and keeps running far away",
            true,
            false,
        )
        .unwrap();
        let lines: Vec<&str> = out.split('\n').collect();
        assert!(lines.len() > 1, "{out}");
        assert!(
            lines.iter().all(|l| text_width(l) <= MD_WRAP_WIDTH),
            "{out}"
        );
        let word = "Supercalifragilisticexpialidociousword";
        assert_eq!(lower(word, true, false).unwrap(), word);
        // Styled words keep their style across the break, and the space
        // between same-styled words joins the run.
        let out = lower("**bold words here** and more", true, false).unwrap();
        assert_eq!(out, "<b>bold words here</b> and more");
    }

    #[test]
    fn open_detection() {
        assert!(leaves_open("A[\"`Line1"));
        assert!(!leaves_open("A[\"`x`\"]"));
        assert!(!leaves_open("A[\"plain `x\"]"));
        assert!(!leaves_open("A[`x`]"));
        assert_eq!(md_quoted(" \"`x`\" "), Some("x"));
        assert_eq!(md_quoted("\"`\""), None);
        assert_eq!(md_quoted("\" `x` \""), None);
    }
}
