import sys
p = sys.argv[1]
src = open(p).read()
old = '''    /// Report Q11: the frame and branch labels had nothing behind
    /// them, so the first lifeline's dashes ran through their glyphs.
    /// Each now sits on a white card as wide as its text.
    #[test]
    fn frame_labels_sit_on_white_cards() {
        let svg = crate::render_svg(include_str!("../examples/sequence.mmd")).unwrap();
        let lines: Vec<&str> = svg.lines().collect();
        for label in ["[session valid]", "[expired]", "[cache warmup]", "[audit]"] {
            let i = lines
                .iter()
                .position(|l| l.starts_with("<text") && l.contains(label))
                .unwrap();
            let card = lines[i - 1];
            assert!(
                card.starts_with("<rect") && card.contains("fill=\\"#ffffff\\""),
                "{label}: {card}"
            );
            let (cx, cw) = (svg_attr(card, "x"), svg_attr(card, "width"));
            let tx = svg_attr(lines[i], "x");
            assert!(
                cx <= tx && tx + text_width(label) <= cx + cw + 0.1,
                "{label}"
            );
        }
    }
'''
new = '''    /// Report Q11: the frame and branch labels had nothing behind
    /// them, so the first lifeline's dashes ran through their glyphs.
    /// Each now sits on a white card as wide as its text (painted
    /// earlier, under the activation bars — see
    /// `label_cards_never_cut_an_activation_bar`).
    #[test]
    fn frame_labels_sit_on_white_cards() {
        let svg = crate::render_svg(include_str!("../examples/sequence.mmd")).unwrap();
        let lines: Vec<&str> = svg.lines().collect();
        for label in ["[session valid]", "[expired]", "[cache warmup]", "[audit]"] {
            let i = lines
                .iter()
                .position(|l| l.starts_with("<text") && l.contains(label))
                .unwrap();
            let (tx, ty) = (svg_attr(lines[i], "x"), svg_attr(lines[i], "y"));
            let card = lines[..i]
                .iter()
                .rev()
                .find(|l| {
                    l.contains("height=\\"16\\" fill=\\"#ffffff\\"")
                        && (svg_attr(l, "y") + 8.0 - ty).abs() < 1e-6
                })
                .unwrap_or_else(|| panic!("{label}: no card painted before it"));
            let (cx, cw) = (svg_attr(card, "x"), svg_attr(card, "width"));
            assert!(
                cx <= tx && tx + text_width(label) <= cx + cw + 0.1,
                "{label}"
            );
        }
    }

    /// Verifier finding on the Q11 label cards: they were painted
    /// AFTER the activation bars, so a bar running under a frame or
    /// divider label got a 16px white gap and read as two separate
    /// activations. Label backgrounds (cards and keyword chips) now go
    /// under the bars and the label text over them: no background may
    /// overlap a bar painted before it, and no bar may cover label
    /// glyphs painted before it. Covers the default bands (the first
    /// participant's bar under the label column), scoped frames (the
    /// leftmost touched column) and nested chips reaching a bar.
    #[test]
    fn label_cards_never_cut_an_activation_bar() {
        let cases = [
            "sequenceDiagram\\nparticipant A\\nparticipant B as Backend\\nparticipant C as Cache\\n\\
             B->>+A: call into A\\nalt cache hit and value fresh\\nA->>C: get\\nC-->>A: value\\n\\
             else cache miss or stale entry\\nA->>B: load\\nB-->>A: data\\nelse\\nA->>A: noop\\nend\\n\\
             A-->>-B: result",
            "---\\nconfig:\\n  sequence:\\n    scopedFrames: true\\n---\\nsequenceDiagram\\n\\
             participant A as Alice\\nparticipant B as Bob\\nparticipant C as Carol\\n\\
             A->>+B: open session\\nalt acquire the lock\\nB->>C: lock\\nC-->>B: granted\\n\\
             else timeout\\nB->>C: retry\\nend\\nloop poll\\nB->>C: status\\nend\\nB-->>-A: done",
            "sequenceDiagram\\nparticipant A\\nparticipant B\\nB->>+A: go\\nloop outer\\nopt mid\\n\\
             loop inner\\npar deep\\npar deeper\\npar deepest\\nA->>B: x\\nend\\nend\\nend\\nend\\nend\\nend\\n\\
             A-->>-B: z",
        ];
        type Rect = (usize, f64, f64, f64, f64);
        let rect = |i: usize, l: &str| -> Rect {
            let (x, y) = (svg_attr(l, "x"), svg_attr(l, "y"));
            (i, x, y, svg_attr(l, "width"), svg_attr(l, "height"))
        };
        let overlap = |a: &Rect, b: &Rect| {
            a.1 < b.1 + b.3 && b.1 < a.1 + a.3 && a.2 < b.2 + b.4 && b.2 < a.2 + a.4
        };
        for src in cases {
            let sc = scene(&sd(src));
            let svg = to_svg(&sc);
            let lines: Vec<&str> = svg.lines().collect();
            let bars: Vec<Rect> = lines
                .iter()
                .enumerate()
                .filter(|(_, l)| {
                    l.starts_with("<rect")
                        && l.contains(&format!("width=\\"{ACT_W:.1}\\""))
                        && l.contains("stroke-width=\\"1.4\\"")
                })
                .map(|(i, l)| rect(i, l))
                .collect();
            assert_eq!(bars.len(), sc.activations.len(), "{src}");
            let backgrounds: Vec<Rect> = lines
                .iter()
                .enumerate()
                .filter(|(_, l)| {
                    l.starts_with("<rect")
                        && (l.contains("height=\\"16\\" fill=\\"#ffffff\\"")
                            || l.contains("fill=\\"#eef1fb\\""))
                })
                .map(|(i, l)| rect(i, l))
                .collect();
            let mut crossed = 0;
            for bg in &backgrounds {
                for bar in bars.iter().filter(|b| overlap(b, bg)) {
                    crossed += 1;
                    assert!(
                        bg.0 < bar.0,
                        "{}\\npainted over the bar\\n{}",
                        lines[bg.0],
                        lines[bar.0]
                    );
                }
            }
            assert!(crossed > 0, "the case must put a label over a bar: {src}");
            // Every frame label / keyword is painted after every bar,
            // so a bar never hides its glyphs.
            let last_bar = bars.iter().map(|b| b.0).max().unwrap();
            for f in &sc.frames {
                let mut texts = vec![format!(">{}</text>", f.kind.keyword())];
                texts.extend(frame_label_rows(f).map(|(_, t)| format!(">{}</text>", escape(&t))));
                for t in texts {
                    let i = lines.iter().position(|l| l.ends_with(&t)).unwrap();
                    assert!(i > last_bar, "{t} painted under a bar");
                }
            }
        }
    }
'''
assert old in src
src = src.replace(old, new)
open(p, 'w').write(src)
