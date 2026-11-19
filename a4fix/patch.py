import sys
p = sys.argv[1]
src = open(p).read()

old_backing = '''/// White card behind a frame / divider label (left-aligned at `x`,
/// vertically centered on `cy`), so the lifelines and bars it crosses
/// don't run through its glyphs. A plain `<rect>` rather than a
/// `paint-order` text halo, which older SVG renderers ignore.
fn label_backing('''
new_backing = '''/// White card behind a frame / divider label (left-aligned at `x`,
/// vertically centered on `cy`), so the lifelines it crosses don't
/// run through its glyphs. A plain `<rect>` rather than a
/// `paint-order` text halo, which older SVG renderers ignore. Painted
/// *under* the activation bars (the label glyphs go over them): a
/// card on top cut a white gap into any bar it crossed, which read as
/// two separate activations.
fn label_backing('''
assert old_backing in src
src = src.replace(old_backing, new_backing)

_a = src.index('    // …activation bars over both (a divider never strikes a bar)…\n'); old_paint = src[_a:src.index('    for nb in &sc.notes {\n', _a)]
new_paint = '''    // …then the label backgrounds: each frame's keyword chip and the
    // white cards under its branch labels…
    for f in &sc.frames {
        s.push_str(&format!(
            "<rect x=\\"{:.1}\\" y=\\"{:.1}\\" width=\\"{:.1}\\" height=\\"18\\" \\
             fill=\\"#eef1fb\\" stroke=\\"{}\\"/>\\n",
            f.x,
            f.y,
            chip_w(f.kind),
            GUIDE_COLOR
        ));
        let lx = frame_label_x(f);
        for (cy, text) in frame_label_rows(f) {
            label_backing(&mut s, lx, cy, &text);
        }
    }
    // …activation bars over all of it (neither a divider nor a label
    // background ever cuts a bar into pieces)…
    for a in &sc.activations {
        s.push_str(&format!(
            "<rect x=\\"{:.1}\\" y=\\"{:.1}\\" width=\\"{:.1}\\" height=\\"{:.1}\\" \\
             fill=\\"#ffffff\\" stroke=\\"{}\\" stroke-width=\\"1.4\\"/>\\n",
            a.x - ACT_W / 2.0,
            a.y0,
            ACT_W,
            a.y1 - a.y0,
            crate::style::accent(a.participant)
        ));
    }
    // …then the chip keywords and branch labels over the bars. All
    // conditions of a frame share one left-aligned column beside the
    // chip.
    for f in &sc.frames {
        s.push_str(&format!(
            "<text x=\\"{:.1}\\" y=\\"{:.1}\\" dy=\\"0.33em\\" text-anchor=\\"middle\\" \\
             font-weight=\\"bold\\" fill=\\"{}\\">{}</text>\\n",
            f.x + chip_w(f.kind) / 2.0,
            f.y + 9.0,
            TEXT_COLOR,
            escape(f.kind.keyword())
        ));
        let lx = frame_label_x(f);
        for (cy, text) in frame_label_rows(f) {
            s.push_str(&format!(
                "<text x=\\"{:.1}\\" y=\\"{:.1}\\" dy=\\"0.33em\\" fill=\\"{}\\">{}</text>\\n",
                lx,
                cy,
                TEXT_COLOR,
                escape(&text)
            ));
        }
    }
'''
src = src.replace(old_paint, new_paint)

old_fl = '''/// Width a frame needs so its chip and its widest branch label stay'''
new_fl = '''/// Every non-empty label of frame `f` as `(center y, "[label]")`:
/// its own label on the chip row, then one per `else` / `and` divider.
fn frame_label_rows(f: &FrameBox) -> impl Iterator<Item = (f64, String)> + '_ {
    std::iter::once((f.y + 9.0, f.label.as_str()))
        .chain(f.dividers.iter().map(|(dy, dl)| (dy + 9.0, dl.as_str())))
        .filter(|(_, label)| !label.is_empty())
        .map(|(cy, label)| (cy, bracketed(label)))
}

/// Width a frame needs so its chip and its widest branch label stay'''
assert old_fl in src
src = src.replace(old_fl, new_fl, 1)
open(p, 'w').write(src)
