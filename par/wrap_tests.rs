
    /// The 13 px text of journeys and timelines, measured as they do.
    fn at_13(t: &str) -> f64 {
        crate::layout::text_width(t) * 13.0 / crate::layout::TEXT_CALIBRATION
    }

    /// Phase A follow-up: journey and timeline text wrapped greedily,
    /// leaving a stranded last word ("… in the | evening"); it is now
    /// balanced like a node label, in as few lines as greedy takes.
    #[test]
    fn wrap_block_leaves_no_stranded_last_word() {
        let s = "Go for a long walk around the lake in the evening";
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
        // A line that fits, and the text's own breaks, stay as written.
        assert_eq!(wrap_block("short  text", 150.0, at_13, true), "short  text");
        assert_eq!(wrap_block("x\ny", 1.0, at_13, true), "x\ny");
        // A word wider than the limit keeps a line to itself, whole.
        let out = wrap_block("a Pneumonoultramicroscopic b", 40.0, at_13, false);
        assert!(
            out.split('\n').any(|l| l.contains("Pneumonoultramicroscopic")),
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
        assert_eq!(plain.join(" "), "Make a before the meeting starts", "{out:?}");
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
