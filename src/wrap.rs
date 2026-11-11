//! Line breaking shared by every label that wraps (report M7): node
//! labels ([`crate::layout::wrap_label`]), markdown strings
//! ([`crate::markdown`]) and the fixed-column text of journeys and
//! timelines ([`wrap_block`]). A label line is cut into
//! [`Token`]s — its words, and each CJK character on its own, since
//! those scripts break between characters without spaces — each token
//! measured once ([`Widths`]: a run of tokens is as wide as their sum
//! and the spaces between them, so choosing breaks costs no measuring);
//! the breakers here only pick where the lines start. Words are never
//! split: a word wider than the limit keeps a line to itself, as
//! mermaid's wrapped labels do.

/// A piece of a label line a line may start at: a word (which may mix
/// styles: `<b>flow</b>maid`), or one CJK character with whatever
/// punctuation may not be parted from it. Where it lies in its line's
/// runs: (run, byte) of its first character and just past its last.
#[derive(Debug, Clone, Copy)]
struct Token {
    from: (usize, usize),
    to: (usize, usize),
    /// The style of the space written before the token; `None` when
    /// none was (the line's first token, or a break between two CJK
    /// characters, which joins them back without one).
    gap: Option<(bool, bool)>,
}

/// Whether a line may break before and after `c` with no space there:
/// Han ideographs, kana and full-width forms, as browsers break them
/// (mermaid breaks a long Japanese label between any two characters).
/// Hangul keeps to its spaces, as Korean is written.
fn breaks_around(c: char) -> bool {
    matches!(c as u32,
        0x3000..=0x303F // CJK symbols and punctuation
        | 0x3040..=0x30FF // hiragana, katakana
        | 0x31F0..=0x31FF // katakana phonetic extensions
        | 0x3400..=0x4DBF // CJK extension A
        | 0x4E00..=0x9FFF // CJK unified ideographs
        | 0xF900..=0xFAFF // CJK compatibility ideographs
        | 0xFF00..=0xFFEF // half- and full-width forms
        | 0x20000..=0x2FA1F) // CJK extensions B on
}

/// Characters a line never starts with (closing punctuation, small
/// kana, the prolonged sound mark: Japanese line-breaking rules), so
/// they stay with the character before them.
fn no_break_before(c: char) -> bool {
    "、。，．・：；？！ー）」』】〕〉》〙〗｝］ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶ々〻ゝゞヽヾ\
     ,.!?:;)]}%"
        .contains(c)
}

/// Characters a line never ends with (opening brackets), so they stay
/// with the character after them.
fn no_break_after(c: char) -> bool {
    "（「『【〔〈《〘〖｛［([{".contains(c)
}

/// A label line's styled runs — (text, bold, italic), as
/// [`crate::layout::spans`] reads `<b>` / `<i>` markup — cut into the
/// tokens a line may start at.
/// Runs of spaces are one break (leading and trailing ones dropped); a
/// CJK character is a token of its own ([`breaks_around`]) but for
/// punctuation that may not be parted from its neighbour. Tokens point
/// into the runs, so cutting and measuring a plain label allocates
/// nothing per word.
pub(crate) struct Tokens<'a, S> {
    runs: &'a [(S, bool, bool)],
    toks: Vec<Token>,
}

impl<'a, S: AsRef<str>> Tokens<'a, S> {
    /// `runs` (one label line) cut into tokens.
    pub(crate) fn new(runs: &'a [(S, bool, bool)]) -> Self {
        let mut toks: Vec<Token> = Vec::new();
        let mut cur: Option<Token> = None;
        // The style of a space since the last token ended.
        let mut gap: Option<(bool, bool)> = None;
        // The last character was CJK: a token may start after it.
        let mut open_after = false;
        let mut last = ' ';
        for (r, (t, b, i)) in runs.iter().enumerate() {
            for (at, c) in t.as_ref().char_indices() {
                if c == ' ' {
                    toks.extend(cur.take());
                    if !toks.is_empty() {
                        gap.get_or_insert((*b, *i));
                    }
                    open_after = false;
                    last = c;
                    continue;
                }
                let end = (r, at + c.len_utf8());
                match &mut cur {
                    Some(tok)
                        if !((breaks_around(c) || open_after)
                            && !no_break_before(c)
                            && !no_break_after(last)) =>
                    {
                        tok.to = end;
                    }
                    _ => {
                        toks.extend(cur.take());
                        cur = Some(Token {
                            from: (r, at),
                            to: end,
                            gap: gap.take(),
                        });
                    }
                }
                open_after = breaks_around(c);
                last = c;
            }
        }
        toks.extend(cur);
        Tokens { runs, toks }
    }

    /// Number of tokens.
    pub(crate) fn len(&self) -> usize {
        self.toks.len()
    }

    /// Token `k`'s styled pieces, in order.
    fn pieces(&self, k: usize) -> impl Iterator<Item = (&'a str, bool, bool)> + '_ {
        let Token { from, to, .. } = self.toks[k];
        let runs = self.runs;
        (from.0..=to.0).filter_map(move |r| {
            let (t, b, i) = &runs[r];
            let t = t.as_ref();
            let a = if r == from.0 { from.1 } else { 0 };
            let z = if r == to.0 { to.1 } else { t.len() };
            (a < z).then(|| (&t[a..z], *b, *i))
        })
    }

    /// Tokens `i..j` on one line, as markup: each token after the first
    /// behind the space it was written with (in that space's style), or
    /// none (a break between CJK characters).
    pub(crate) fn line(&self, i: usize, j: usize) -> String {
        let mut out = Markup::default();
        for k in i..j {
            if let (true, Some((b, it))) = (k > i, self.toks[k].gap) {
                out.push(" ", b, it);
            }
            for (t, b, it) in self.pieces(k) {
                out.push(t, b, it);
            }
        }
        out.finish()
    }

    /// The lines starting at `starts` (see [`balanced`]), as markup.
    pub(crate) fn lines(&self, starts: &[usize]) -> Vec<String> {
        starts
            .iter()
            .enumerate()
            .map(|(k, &s)| self.line(s, starts.get(k + 1).copied().unwrap_or(self.len())))
            .collect()
    }

    /// Each token measured with `measure` ([`Widths`]).
    pub(crate) fn widths(&self, measure: impl Fn(&str) -> f64) -> Widths {
        // A space measured between two letters, so a measure that trims
        // a lone space still counts it.
        let space = (measure("x x") - 2.0 * measure("x")).max(0.0);
        let n = self.len();
        let mut pre = Vec::with_capacity(n + 1);
        let mut gap = Vec::with_capacity(n);
        let mut widest: f64 = 0.0;
        pre.push(0.0);
        for k in 0..n {
            let g = if k > 0 && self.toks[k].gap.is_some() {
                space
            } else {
                0.0
            };
            let mut ps = self.pieces(k);
            let w = match (ps.next(), ps.next()) {
                (Some((t, false, false)), None) => measure(t),
                _ => measure(&self.line(k, k + 1)),
            };
            widest = widest.max(w);
            gap.push(g);
            pre.push(pre[k] + g + w);
        }
        Widths { pre, gap, widest }
    }
}

/// Label markup written run by run: a run in another style than the
/// last closes its tags and opens its own, so consecutive runs of one
/// style share their tags (bold `<b>…</b>`, italic `<i>…</i>`, both
/// `<b><i>…</i></b>`).
#[derive(Default)]
struct Markup {
    out: String,
    style: (bool, bool),
}

impl Markup {
    fn close(&mut self) {
        if self.style.1 {
            self.out.push_str("</i>");
        }
        if self.style.0 {
            self.out.push_str("</b>");
        }
    }

    fn push(&mut self, t: &str, b: bool, i: bool) {
        if (b, i) != self.style {
            self.close();
            if b {
                self.out.push_str("<b>");
            }
            if i {
                self.out.push_str("<i>");
            }
            self.style = (b, i);
        }
        self.out.push_str(t);
    }

    fn finish(mut self) -> String {
        self.close();
        self.out
    }
}

/// `runs` as label markup (see [`Markup`]).
pub(crate) fn markup<S: AsRef<str>>(runs: &[(S, bool, bool)]) -> String {
    let mut out = Markup::default();
    for (t, b, i) in runs {
        out.push(t.as_ref(), *b, *i);
    }
    out.finish()
}

/// Each token's width, measured once, and how wide any run of them is
/// on one line: the tokens' widths plus a space's for each space
/// between them. Exact for [`crate::layout::text_width`], which sums
/// per character; for an embedder's real font it leaves out kerning
/// across a space, a fraction of a pixel, and the box is still sized
/// on the chosen lines measured whole.
pub(crate) struct Widths {
    /// `pre[k]`: tokens `0..k` with the space before each but the first.
    pre: Vec<f64>,
    /// The space before token `k` (0 for none, and for token 0).
    gap: Vec<f64>,
    widest: f64,
}

impl Widths {
    /// Number of tokens.
    pub(crate) fn len(&self) -> usize {
        self.gap.len()
    }

    /// Width of tokens `i..j` on one line.
    pub(crate) fn line(&self, i: usize, j: usize) -> f64 {
        self.pre[j] - self.pre[i] - self.gap[i]
    }

    /// Widest of the lines starting at `starts`.
    pub(crate) fn widest_line(&self, starts: &[usize]) -> f64 {
        (0..starts.len())
            .map(|k| self.line(starts[k], starts.get(k + 1).copied().unwrap_or(self.len())))
            .fold(0.0, f64::max)
    }
}

/// Where the lines of a label measured by `w` start when it wraps at
/// `width`: [`balanced`], with two allowances for a token wider than
/// `width` (which sets the block's width whatever the breaks). The
/// other lines may fill up to it, as the node is that wide anyway; and
/// a lone token left on the first or last line beside such a word joins
/// it ("…_antidisestablishmentarianism word" on one line, not the word
/// stranded under it).
pub(crate) fn wrap(w: &Widths, width: f64) -> Vec<usize> {
    let n = w.len();
    let room = width.max(w.widest);
    let mut starts = balanced(n, room, |i, j| w.line(i, j));
    if room > width {
        // A lone token no wider than `width` beside a line wider than it.
        let lone = |s: usize| w.line(s, s + 1) <= width;
        let k = starts.len();
        if k >= 2
            && n - starts[k - 1] == 1
            && lone(n - 1)
            && w.line(starts[k - 2], starts[k - 1]) > width
        {
            starts.pop();
        }
        if starts.len() >= 2 && starts[1] == 1 && lone(0) {
            let next = starts.get(2).copied().unwrap_or(n);
            if w.line(1, next) > width {
                starts.remove(1);
            }
        }
    }
    starts
}

/// `s` wrapped at `width` for the fixed-column diagrams — journey task
/// names, timeline periods and events — whose text would otherwise
/// widen its column without bound: each of its own lines (`\n`) that
/// `measure` finds wider than `width` broken as a node label is
/// ([`wrap`]: as few lines as filling them greedily needs, evened out so
/// no word is left stranded on the last — "Sit down and | write the
/// weekly | report for the team", not "… weekly report for the | team"),
/// a word wider than `width` kept whole on a line of its own. Lines
/// that fit come back as written.
///
/// `styled`: the text's `<b>` / `<i>` markup is drawn as styling
/// ([`crate::layout::spans`]), so it is kept and re-opened on every
/// line a styled run reaches (a greedy cut used to leave the run's
/// second line plain). Otherwise the tags are text like any other (a
/// journey prints them verbatim) and are measured and broken as such.
/// A line holding `$$…$$` math breaks as plain text, its fences kept.
pub(crate) fn wrap_block(
    s: &str,
    width: f64,
    measure: impl Fn(&str) -> f64,
    styled: bool,
) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in s.split('\n') {
        if measure(line) <= width {
            out.push(line.to_string());
        } else if styled && line.contains('<') && !line.contains("$$") {
            out.extend(wrap_runs(&crate::layout::spans(line), width, &measure));
        } else {
            out.extend(wrap_runs(&[(line, false, false)], width, &measure));
        }
    }
    out.join("\n")
}

/// The styled `runs` of one line broken at `width` ([`wrap_block`]), as
/// one markup string per line.
fn wrap_runs<S: AsRef<str>>(
    runs: &[(S, bool, bool)],
    width: f64,
    measure: &impl Fn(&str) -> f64,
) -> Vec<String> {
    let toks = Tokens::new(runs);
    if toks.len() == 0 {
        // Nothing but spaces: no word to start a line at.
        return vec![markup(runs)];
    }
    let w = toks.widths(measure);
    toks.lines(&wrap(&w, width))
}

/// Most words [`balanced`] balances: past it a label is a paragraph,
/// not a label, and greedy filling (linear) is good enough.
const BALANCE_MAX_WORDS: usize = 64;

/// Where the lines of `n` words start, filling each line greedily while
/// the next word fits in `width` (`line_w(i, j)`: how wide words `i..j`
/// are on one line). The first entry is always 0; empty for no words.
fn greedy(n: usize, width: f64, line_w: impl Fn(usize, usize) -> f64) -> Vec<usize> {
    let mut starts = Vec::new();
    if n == 0 {
        return starts;
    }
    starts.push(0);
    let mut from = 0;
    for j in 1..n {
        if line_w(from, j + 1) > width {
            starts.push(j);
            from = j;
        }
    }
    starts
}

/// Where the lines of `n` words start when they wrap at `width`
/// balanced: as few lines as [`greedy`] filling needs, and of all the
/// ways to break into that many lines (none wider than `width` but for
/// a lone over-long word) the one whose widest line is narrowest, the
/// others as even as that allows, ties to the earliest breaks. So a label never ends on a stranded short
/// word when moving words down evens the lines out ("Return documents
/// to | the applicant", not "Return documents to the | applicant").
/// Over [`BALANCE_MAX_WORDS`] words, greedy.
pub(crate) fn balanced(n: usize, width: f64, line_w: impl Fn(usize, usize) -> f64) -> Vec<usize> {
    let starts = greedy(n, width, &line_w);
    let k = starts.len();
    if k <= 1 || n > BALANCE_MAX_WORDS {
        return starts;
    }
    into_lines(n, k, width, &line_w).unwrap_or(starts)
}

/// Where the lines of `n` words start when broken into exactly `k`
/// lines, the widest as narrow as can be and the rest as even as that
/// allows (ties to the earliest breaks),
/// no line wider than `width` but for a lone word (pass infinity for no
/// limit); `None` when no such break exists (`k` above `n`, or too few
/// lines for `width`), or past [`BALANCE_MAX_WORDS`] words.
pub(crate) fn into_lines(
    n: usize,
    k: usize,
    width: f64,
    line_w: impl Fn(usize, usize) -> f64,
) -> Option<Vec<usize>> {
    if k == 0 || k > n || n > BALANCE_MAX_WORDS {
        return None;
    }
    // Width of words i..j on one line, or None when it overflows (a
    // lone word may): the lines ending at `j` start no further back
    // than the first that overflows, so each step looks at a line's
    // worth of words, not all of them.
    let fits = |i: usize, j: usize| {
        let lw = line_w(i, j);
        (lw <= width || j == i + 1).then_some(lw)
    };
    // The words `m` lines can hold: no more than `m` lines filled from
    // the left (`most[m]`), no fewer than leave the rest to fit in the
    // other `k - m` filled from the right (`least[k - m]`). Only that
    // band of each row can be on a break, so the tables are filled
    // there alone.
    let mut most = vec![0usize; k + 1];
    for m in 1..=k {
        let i = most[m - 1];
        let mut j = (i + 1).min(n);
        while j < n && fits(i, j + 1).is_some() {
            j += 1;
        }
        most[m] = j;
    }
    let mut least = vec![n; k + 1];
    for t in 1..=k {
        let j = least[t - 1];
        let mut i = j.saturating_sub(1);
        while i > 0 && fits(i - 1, j).is_some() {
            i -= 1;
        }
        least[t] = i;
    }
    let band = |m: usize| m.max(least[k - m])..=most[m];
    // best[m][j]: narrowest widest line over words 0..j in m lines.
    // (Both tables are flat, row `m` at `m * (n + 1)`.)
    let at = |m: usize, j: usize| m * (n + 1) + j;
    let mut best = vec![f64::INFINITY; (k + 1) * (n + 1)];
    best[0] = 0.0;
    for m in 1..=k {
        for j in band(m) {
            for i in (m - 1..j).rev() {
                let Some(lw) = fits(i, j) else { break };
                let prev = best[at(m - 1, i)];
                if !prev.is_finite() {
                    continue;
                }
                let cand = prev.max(lw);
                if cand < best[at(m, j)] {
                    best[at(m, j)] = cand;
                }
            }
        }
    }
    let widest = best[at(k, n)];
    if !widest.is_finite() {
        return None;
    }
    // Of the breaks whose widest line is that narrow, the most even:
    // least summed squared shortfall of each line from the widest
    // ("Are all documents | complete and | compliant with | the
    // rules?", not "… | complete | and compliant | …").
    let cap = widest + 1e-9;
    // even[m][j]: least shortfall over words 0..j in m lines, and where
    // its last line starts.
    let mut even = vec![(f64::INFINITY, 0usize); (k + 1) * (n + 1)];
    even[0] = (0.0, 0);
    for m in 1..=k {
        for j in band(m) {
            for i in (m - 1..j).rev() {
                let Some(lw) = fits(i, j) else { break };
                let prev = even[at(m - 1, i)].0;
                if !prev.is_finite() || lw > cap {
                    continue;
                }
                let cand = prev + (widest - lw).powi(2);
                // `<=` walking back: ties go to the earliest break.
                if cand <= even[at(m, j)].0 {
                    even[at(m, j)] = (cand, i);
                }
            }
        }
    }
    let mut out = vec![0; k];
    let mut j = n;
    for m in (1..=k).rev() {
        let i = even[at(m, j)].1;
        out[m - 1] = i;
        j = i;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words of the given widths, one px per space between them.
    fn widths(ws: &[f64]) -> impl Fn(usize, usize) -> f64 + '_ {
        move |i, j| ws[i..j].iter().sum::<f64>() + (j - i - 1) as f64
    }

    /// The plain two-pass DP over every break, which [`into_lines`]
    /// must agree with: it only fills the band of words each line count
    /// can hold.
    fn naive_into_lines(
        n: usize,
        k: usize,
        width: f64,
        line_w: &dyn Fn(usize, usize) -> f64,
    ) -> Option<Vec<usize>> {
        let w = |i: usize, j: usize| {
            let lw = line_w(i, j);
            if lw <= width || j == i + 1 {
                lw
            } else {
                f64::INFINITY
            }
        };
        let mut best = vec![vec![f64::INFINITY; n + 1]; k + 1];
        best[0][0] = 0.0;
        for m in 1..=k {
            for j in m..=n {
                for i in (m - 1)..j {
                    let c = best[m - 1][i].max(w(i, j));
                    if c < best[m][j] {
                        best[m][j] = c;
                    }
                }
            }
        }
        let widest = best[k][n];
        if !widest.is_finite() {
            return None;
        }
        let mut even = vec![vec![(f64::INFINITY, 0usize); n + 1]; k + 1];
        even[0][0] = (0.0, 0);
        for m in 1..=k {
            for j in m..=n {
                for i in (m - 1)..j {
                    let lw = w(i, j);
                    if lw > widest + 1e-9 || !even[m - 1][i].0.is_finite() {
                        continue;
                    }
                    let c = even[m - 1][i].0 + (widest - lw).powi(2);
                    if c < even[m][j].0 {
                        even[m][j] = (c, i);
                    }
                }
            }
        }
        let mut out = vec![0; k];
        let mut j = n;
        for m in (1..=k).rev() {
            out[m - 1] = even[m][j].1;
            j = out[m - 1];
        }
        Some(out)
    }

    #[test]
    fn into_lines_agrees_with_the_full_search() {
        // Deterministic pseudo-random word widths, lone over-long words
        // included.
        let mut seed = 0x2545_f491_u64;
        let mut next = |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        for _ in 0..300 {
            let n = 1 + next(30) as usize;
            let ws: Vec<f64> = (0..n).map(|_| 5.0 + next(90) as f64).collect();
            let width = 40.0 + next(160) as f64;
            let lw = widths(&ws);
            for k in 1..=n.min(12) {
                assert_eq!(
                    into_lines(n, k, width, &lw),
                    naive_into_lines(n, k, width, &lw),
                    "{ws:?} at {width} in {k}"
                );
            }
        }
    }

    #[test]
    fn greedy_fills_lines_and_keeps_long_words_whole() {
        assert_eq!(greedy(0, 10.0, |_, _| 0.0), Vec::<usize>::new());
        assert_eq!(greedy(3, 100.0, widths(&[10.0, 10.0, 10.0])), vec![0]);
        assert_eq!(greedy(3, 21.0, widths(&[10.0, 10.0, 10.0])), vec![0, 2]);
        // An over-long word sits alone, unsplit.
        assert_eq!(greedy(3, 15.0, widths(&[10.0, 50.0, 10.0])), vec![0, 1, 2]);
    }

    #[test]
    fn balanced_keeps_the_line_count_and_evens_the_lines() {
        // Greedy strands the last word: [30 30 30] [10]; balanced moves
        // a word down: [30 30] [30 10].
        let ws = [30.0, 30.0, 30.0, 10.0];
        assert_eq!(greedy(4, 95.0, widths(&ws)), vec![0, 3]);
        assert_eq!(balanced(4, 95.0, widths(&ws)), vec![0, 2]);
        // One line when it fits; a lone over-long word stays alone.
        assert_eq!(balanced(4, 200.0, widths(&ws)), vec![0]);
        assert_eq!(
            balanced(3, 20.0, widths(&[10.0, 50.0, 10.0])),
            vec![0, 1, 2]
        );
        // Exactly k lines, the widest as narrow as can be.
        assert_eq!(
            into_lines(4, 3, f64::INFINITY, widths(&ws)),
            Some(vec![0, 1, 2])
        );
        assert_eq!(into_lines(4, 5, f64::INFINITY, widths(&ws)), None);
        assert_eq!(into_lines(4, 1, 50.0, widths(&ws)), None);
        // Never more lines than greedy, never a line over the width but
        // a lone word.
        let ws = [12.0, 40.0, 7.0, 33.0, 21.0, 5.0, 60.0, 9.0];
        for width in [40.0, 55.0, 70.0, 90.0, 130.0] {
            let g = greedy(ws.len(), width, widths(&ws));
            let b = balanced(ws.len(), width, widths(&ws));
            assert_eq!(b.len(), g.len(), "{width}");
            let ends: Vec<usize> = b.iter().skip(1).copied().chain([ws.len()]).collect();
            for (&s, &e) in b.iter().zip(&ends) {
                assert!(e > s);
                assert!(
                    e - s == 1 || widths(&ws)(s, e) <= width,
                    "{width}: {s}..{e}"
                );
            }
        }
    }

    /// The 13 px text of journeys and timelines, measured as they do.
    fn at_13(t: &str) -> f64 {
        crate::layout::text_width(t) * 13.0 / crate::layout::TEXT_CALIBRATION
    }

    /// Phase A follow-up: journey and timeline text wrapped greedily,
    /// leaving a stranded last word ("… report for the | team"); it is now
    /// balanced like a node label, in as few lines as greedy takes.
    #[test]
    fn wrap_block_leaves_no_stranded_last_word() {
        let s = "Sit down and write the weekly report for the team";
        let words: Vec<&str> = s.split(' ').collect();
        let line = |i: usize, j: usize| at_13(&words[i..j].join(" "));
        let g = greedy(words.len(), 150.0, line);
        let out = wrap_block(s, 150.0, at_13, false);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), g.len(), "{out:?}");
        assert_eq!(lines.join(" "), s, "no word lost or reordered");
        // Greedy strands a short tail; balanced evens the lines out.
        let spread = |ws: &[f64]| {
            ws.iter().cloned().fold(0.0, f64::max) - ws.iter().cloned().fold(f64::MAX, f64::min)
        };
        let ends = |st: &[usize]| -> Vec<f64> {
            (0..st.len())
                .map(|k| line(st[k], st.get(k + 1).copied().unwrap_or(words.len())))
                .collect()
        };
        let ws: Vec<f64> = lines.iter().map(|l| at_13(l)).collect();
        for w in &ws {
            assert!(*w <= 150.0, "{out:?}");
        }
        assert!(spread(&ws) < spread(&ends(&g)), "{out:?} vs greedy {g:?}");
        assert_ne!(*lines.last().unwrap(), "team", "{out:?}");
        // A line that fits, and the text's own breaks, stay as written.
        assert_eq!(wrap_block("short  text", 150.0, at_13, true), "short  text");
        assert_eq!(wrap_block("x\ny", 1.0, at_13, true), "x\ny");
        // A word wider than the limit keeps a line to itself, whole.
        let out = wrap_block("a Pneumonoultramicroscopic b", 40.0, at_13, false);
        assert!(
            out.split('\n')
                .any(|l| l.contains("Pneumonoultramicroscopic")),
            "{out:?}"
        );
    }

    /// Phase A follow-up: a `<b>` / `<i>` run cut by a wrap kept its
    /// style on its first line only (each drawn line is read on its
    /// own). Styled text re-opens the run on every line it reaches.
    #[test]
    fn wrap_block_carries_styles_across_its_breaks() {
        let s = "Make a <b>very strong cup of coffee</b> before the meeting starts";
        let out = wrap_block(s, 160.0, at_13, true);
        let lines: Vec<&str> = out.split('\n').collect();
        assert!(lines.len() >= 2, "{out:?}");
        let mut bold = Vec::new();
        let mut plain = Vec::new();
        for l in &lines {
            assert!(at_13(l) <= 160.0, "{l:?}");
            for (t, b, i) in crate::layout::spans(l) {
                assert!(!i);
                let to = if b { &mut bold } else { &mut plain };
                to.extend(t.split(' ').filter(|w| !w.is_empty()).map(str::to_string));
            }
        }
        assert_eq!(bold.join(" "), "very strong cup of coffee", "{out:?}");
        assert_eq!(
            plain.join(" "),
            "Make a before the meeting starts",
            "{out:?}"
        );
        // The run spans a break: each line it reaches opens its own tag.
        assert!(
            lines.iter().filter(|l| l.contains("<b>")).count() >= 2,
            "{out:?}"
        );
        let it = wrap_block(
            "A <i>rather long period name that wraps</i> twice",
            160.0,
            at_13,
            true,
        );
        assert!(it.contains('\n'), "{it:?}");
        assert!(it.split('\n').all(|l| l.contains("<i>")), "{it:?}");
    }

    /// Unstyled (journey) text keeps its tags as text: measured and
    /// broken like any other characters, never re-opened.
    #[test]
    fn wrap_block_keeps_literal_tags_as_text() {
        let width = |t: &str| t.chars().count() as f64;
        let out = wrap_block("make a <b>very strong cup</b> of tea", 14.0, width, false);
        assert!(out.contains('\n'), "{out:?}");
        assert_eq!(
            out.replace('\n', " "),
            "make a <b>very strong cup</b> of tea"
        );
        assert_eq!(out.matches("<b>").count(), 1, "{out:?}");
        for l in out.split('\n') {
            assert!(width(l) <= 14.0, "{l:?}");
        }
        // Math keeps its fences when its line breaks.
        let out = wrap_block(
            "energy is $$E=mc^2$$ for a body at rest",
            120.0,
            at_13,
            true,
        );
        assert!(out.contains('\n') && out.contains("$$E=mc^2$$"), "{out:?}");
    }
}
