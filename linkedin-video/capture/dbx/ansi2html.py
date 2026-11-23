#!/usr/bin/env python3
"""Convert `tmux capture-pane -e -p` output (SGR incl. 24-bit) to an HTML page
that looks like a terminal window titled "dbx".

Text goes into one absolutely positioned row per terminal line (so nothing can
drift), non-ASCII glyphs sit in fixed-width cells, and braille cells
(U+2800..U+28FF) are painted as dots on an SVG layer at their exact cell
positions -- the way terminals such as kitty / WezTerm / Ghostty draw braille
themselves instead of relying on a font that may not have the glyphs.

usage: ansi2html.py in.ansi out.html COLS ROWS [FS LH W H TITLE]
"""
import html
import re
import sys
import unicodedata

PALETTE = [
    "#1d2030", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#c0caf5",
    "#5b6385", "#ff8fa3", "#b9f27c", "#ffc777", "#82aaff", "#c7a9ff", "#a4e4ff", "#eef1ff",
]
BG = "#0d1117"   # dbx dark theme background
FG = "#e6edf3"   # dbx dark theme text


def xterm256(n):
    if n < 16:
        return PALETTE[n]
    if n < 232:
        n -= 16
        r, g, b = n // 36, (n // 6) % 6, n % 6
        lv = [0, 95, 135, 175, 215, 255]
        return "#%02x%02x%02x" % (lv[r], lv[g], lv[b])
    v = 8 + (n - 232) * 10
    return "#%02x%02x%02x" % (v, v, v)


class St:
    def __init__(self):
        self.reset()

    def reset(self):
        self.fg = None
        self.bg = None
        self.bold = False
        self.dim = False
        self.italic = False
        self.under = False
        self.rev = False

    def key(self):
        return (self.fg, self.bg, self.bold, self.dim, self.italic, self.under, self.rev)


def apply_sgr(st, params):
    p = [int(x) if x else 0 for x in params.split(";")] if params else [0]
    i = 0
    while i < len(p):
        c = p[i]
        if c == 0:
            st.reset()
        elif c == 1:
            st.bold = True
        elif c == 2:
            st.dim = True
        elif c == 3:
            st.italic = True
        elif c == 4:
            st.under = True
        elif c == 7:
            st.rev = True
        elif c == 22:
            st.bold = st.dim = False
        elif c == 23:
            st.italic = False
        elif c == 24:
            st.under = False
        elif c == 27:
            st.rev = False
        elif 30 <= c <= 37:
            st.fg = PALETTE[c - 30]
        elif c == 39:
            st.fg = None
        elif 40 <= c <= 47:
            st.bg = PALETTE[c - 40]
        elif c == 49:
            st.bg = None
        elif 90 <= c <= 97:
            st.fg = PALETTE[c - 90 + 8]
        elif 100 <= c <= 107:
            st.bg = PALETTE[c - 100 + 8]
        elif c in (38, 48):
            if i + 1 < len(p) and p[i + 1] == 5 and i + 2 < len(p):
                col = xterm256(p[i + 2])
                i += 2
            elif i + 1 < len(p) and p[i + 1] == 2 and i + 4 < len(p):
                col = "#%02x%02x%02x" % (p[i + 2], p[i + 3], p[i + 4])
                i += 4
            else:
                col = None
            if c == 38:
                st.fg = col
            else:
                st.bg = col
        i += 1


SGR = re.compile(r"\x1b\[([0-9;:]*)m")
OTHER = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b[()][0-9A-Za-z]|\x1b[=>]")
EMPTY = (None, None, False, False, False, False, False)
ZERO_WIDTH = {"\ufe0f", "\ufe0e", "\u200d", "\u200b"}
WIDE_TAIL = ""  # placeholder for the second cell of a wide glyph


def cell_width(ch):
    return 2 if unicodedata.east_asian_width(ch[0]) in ("W", "F") else 1


def parse(text, cols, rows):
    st = St()
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines = lines[:-1]
    grid = []
    for raw in lines[:rows]:
        cells = []
        pos = 0
        skip_next = [False]
        while pos < len(raw):
            m = SGR.match(raw, pos)
            if m:
                apply_sgr(st, m.group(1).replace(":", ";"))
                pos = m.end()
                continue
            m = OTHER.match(raw, pos)
            if m:
                pos = m.end()
                continue
            ch = raw[pos]
            pos += 1
            if unicodedata.combining(ch) or ch in ZERO_WIDTH:
                # attaches to the previous cell (e.g. VS16 after an emoji)
                if cells:
                    pch, pk = cells[-1]
                    cells[-1] = (pch + ch, pk)
                    if ch == "\ufe0f" and cell_width(pch) == 1:
                        # tmux 3.4 counts an emoji + VS16 as one cell, but the
                        # app (ratatui / unicode-width) and modern terminals
                        # draw it two cells wide and never write the second
                        # cell. Render it as the app intends: two cells, and
                        # drop the stale cell tmux keeps after it.
                        cells.append((WIDE_TAIL, pk))
                        skip_next[0] = True
                continue
            if skip_next[0]:
                skip_next[0] = False
                continue
            cells.append((ch, st.key()))
            if cell_width(ch) == 2:
                cells.append((WIDE_TAIL, st.key()))
        while len(cells) < cols:
            cells.append((" ", EMPTY))
        grid.append(cells[:cols])
    while len(grid) < rows:
        grid.append([(" ", EMPTY)] * cols)
    return grid


def colours(k):
    fg, bg, bold, dim, italic, under, rev = k
    f = fg or FG
    b = bg
    if rev:
        f, b = (bg or BG), (fg or FG)
    return f, b


def span(text, k, fixed=False):
    fg, bg, bold, dim, italic, under, rev = k
    f, b = colours(k)
    css = ["color:" + f]
    if b:
        css.append("background:" + b)
    if bold:
        css.append("font-weight:700")
    if dim:
        css.append("opacity:.6")
    if italic:
        css.append("font-style:italic")
    if under:
        css.append("text-decoration:underline")
    cls = (' class="c"' if fixed == 1 else ' class="c2"') if fixed else ""
    return '<span%s style="%s">%s</span>' % (cls, ";".join(css), html.escape(text))


# Braille dot bit -> (column, row) inside the 2x4 cell.
BRAILLE_BITS = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3)]


def render(grid, cols, rows, fs, lh, W, H, title):
    barh = 32
    cw = fs * 0.6
    padx = (W - cols * cw) / 2
    pady = (H - barh - rows * lh) / 2
    rows_html = []
    dots = []
    for r, cells in enumerate(grid):
        parts = []
        cur, buf = None, ""

        def flush():
            if buf:
                parts.append(span(buf, cur))

        for c, (ch, k) in enumerate(cells):
            if ch == WIDE_TAIL:
                continue
            o = ord(ch[0])
            if 0x2800 <= o <= 0x28FF:
                bits = o - 0x2800
                f, b = colours(k)
                for bit, (dx, dy) in enumerate(BRAILLE_BITS):
                    if bits & (1 << bit):
                        x = padx + c * cw + cw * (0.25 + 0.5 * dx)
                        y = pady + r * lh + lh * (0.125 + 0.25 * dy)
                        dots.append((x, y, f))
                ch = " "  # keep the cell (and its background) in the text layer
            if o > 0x7F and not (0x2800 <= o <= 0x28FF) and not (0x2500 <= o <= 0x257F):
                # non-ASCII glyph: pin it to exactly one cell (two if wide)
                if buf:
                    parts.append(span(buf, cur))
                wide = c + 1 < len(cells) and cells[c + 1][0] == WIDE_TAIL
                parts.append(span(ch, k, fixed=2 if wide else 1))
                cur, buf = None, ""
                continue
            if k != cur:
                if buf:
                    parts.append(span(buf, cur))
                cur, buf = k, ""
            buf += ch
        if buf:
            parts.append(span(buf, cur))
        rows_html.append('<div class="row" style="top:%.2fpx">%s</div>' % (pady + r * lh, "".join(parts)))
    rad = min(cw / 2, lh / 4) * 0.42
    svg = ['<svg class="ov" width="%d" height="%d" viewBox="0 0 %d %d">' % (W, H - barh, W, H - barh)]
    by_col = {}
    for x, y, f in dots:
        by_col.setdefault(f, []).append((x, y))
    for f, pts in by_col.items():
        svg.append('<g fill="%s">' % f)
        svg.extend('<circle cx="%.2f" cy="%.2f" r="%.2f"/>' % (x, y, rad) for x, y in pts)
        svg.append("</g>")
    svg.append("</svg>")
    return PAGE % dict(EFS=fs * 0.92, W=W, H=H, BG=BG, FG=FG, BARH=barh, FS=fs, LH=lh, CW=cw, PADX=padx,
                       ROWS="\n".join(rows_html), SVG="".join(svg), TITLE=html.escape(title))


PAGE = """<!doctype html><html><head><meta charset="utf-8">
<style>
html,body{margin:0;padding:0;background:#05070a;}
.win{position:absolute;left:0;top:0;width:%(W)dpx;height:%(H)dpx;background:%(BG)s;overflow:hidden;}
.bar{height:%(BARH)dpx;background:#1b2028;display:flex;align-items:center;position:relative;border-bottom:1px solid #2a313c;box-sizing:border-box;}
.dot{width:12px;height:12px;border-radius:50%%;margin-left:8px;}
.dot:first-child{margin-left:14px;}
.ttl{position:absolute;left:0;right:0;text-align:center;font:500 14px Inter,"DejaVu Sans",sans-serif;color:#aab4c3;letter-spacing:.02em;}
.term{position:absolute;left:0;top:%(BARH)dpx;width:%(W)dpx;height:calc(%(H)dpx - %(BARH)dpx);}
.row{position:absolute;left:%(PADX).2fpx;height:%(LH)spx;line-height:%(LH)spx;white-space:pre;
 font-family:"JetBrains Mono","DejaVu Sans Mono",monospace;font-size:%(FS)spx;color:%(FG)s;
 font-variant-ligatures:none;font-feature-settings:"liga" 0,"calt" 0;}
.c{display:inline-block;width:%(CW).3fpx;overflow:visible;text-align:center;}
.c2{display:inline-block;width:calc(2 * %(CW).3fpx);overflow:visible;text-align:center;font-family:"Noto Color Emoji","JetBrains Mono",monospace;font-size:%(EFS).1fpx;}
.ov{position:absolute;left:0;top:0;}
</style></head><body><div class="win"><div class="bar"><div class="dot" style="background:#ff5f57"></div><div class="dot" style="background:#febc2e"></div><div class="dot" style="background:#28c840"></div><div class="ttl">%(TITLE)s</div></div>
<div class="term">%(ROWS)s
%(SVG)s</div></div></body></html>"""


def main():
    src, dst, cols, rows = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    fs = float(sys.argv[5]) if len(sys.argv) > 5 else 15
    lh = float(sys.argv[6]) if len(sys.argv) > 6 else 20
    W = int(sys.argv[7]) if len(sys.argv) > 7 else 1120
    H = int(sys.argv[8]) if len(sys.argv) > 8 else 880
    title = sys.argv[9] if len(sys.argv) > 9 else "dbx"
    text = open(src, encoding="utf-8", errors="replace").read()
    grid = parse(text, cols, rows)
    open(dst, "w", encoding="utf-8").write(render(grid, cols, rows, fs, lh, W, H, title))


if __name__ == "__main__":
    main()
