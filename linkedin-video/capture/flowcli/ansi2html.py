#!/usr/bin/env python3
"""Convert `tmux capture-pane -e -p` output (SGR incl. 24-bit) to an HTML page.

usage: ansi2html.py in.ansi out.html COLS ROWS
"""
import html
import re
import sys

# Dark terminal palette (16 colours). Index: 0-7 normal, 8-15 bright.
PALETTE = [
    "#1d2030",  # 0 black
    "#f7768e",  # 1 red
    "#9ece6a",  # 2 green
    "#e0af68",  # 3 yellow
    "#7aa2f7",  # 4 blue
    "#bb9af7",  # 5 magenta
    "#7dcfff",  # 6 cyan
    "#c0caf5",  # 7 white (Gray)
    "#5b6385",  # 8 bright black (DarkGray)
    "#ff8fa3",  # 9
    "#b9f27c",  # 10
    "#ffc777",  # 11
    "#82aaff",  # 12 (LightBlue)
    "#c7a9ff",  # 13
    "#a4e4ff",  # 14
    "#eef1ff",  # 15 bright white (White)
]
BG = "#13151f"
FG = "#c0caf5"


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


def convert(text, cols, rows, cursor=None):
    st = St()
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines = lines[:-1]
    out_rows = []
    rgb_seen = set()
    for raw in lines[:rows]:
        cells = []  # (char, key)
        pos = 0
        while pos < len(raw):
            m = SGR.match(raw, pos)
            if m:
                prm = m.group(1).replace(":", ";")
                if "38;2" in prm or "48;2" in prm:
                    rgb_seen.add(prm)
                apply_sgr(st, prm)
                pos = m.end()
                continue
            m = OTHER.match(raw, pos)
            if m:
                pos = m.end()
                continue
            ch = raw[pos]
            pos += 1
            cells.append((ch, st.key()))
        # pad to full width
        while len(cells) < cols:
            cells.append((" ", (None, None, False, False, False, False, False)))
        out_rows.append(cells[:cols])
        # tmux -e resets at line end? keep state across lines like a terminal
    while len(out_rows) < rows:
        out_rows.append([(" ", (None, None, False, False, False, False, False))] * cols)
    if cursor and 0 <= cursor[1] < rows and 0 <= cursor[0] < cols:
        row = list(out_rows[cursor[1]])
        ch, k = row[cursor[0]]
        row[cursor[0]] = (ch, ("#13151f", "#c0caf5") + k[2:])
        out_rows[cursor[1]] = row

    parts = []
    for cells in out_rows:
        line = []
        cur = None
        buf = ""
        for ch, k in cells:
            if k != cur:
                if buf:
                    line.append(span(buf, cur))
                cur, buf = k, ""
            buf += ch
        if buf:
            line.append(span(buf, cur))
        parts.append("".join(line))
    return "\n".join(parts), rgb_seen


def span(text, k):
    fg, bg, bold, dim, italic, under, rev = k
    f = fg or FG
    b = bg
    if rev:
        f, b = (bg or BG), (fg or FG)
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
    return '<span style="%s">%s</span>' % (";".join(css), html.escape(text))


PAGE = """<!doctype html><html><head><meta charset="utf-8">
<style>
html,body{margin:0;padding:0;background:#0b0c12;}
.win{position:absolute;left:0;top:0;width:%(W)dpx;height:%(H)dpx;background:%(BG)s;overflow:hidden;}
.bar{height:%(BARH)dpx;background:#1f2233;display:flex;align-items:center;position:relative;border-bottom:1px solid #2a2e44;}
.dot{width:12px;height:12px;border-radius:50%%;margin-left:8px;}
.dot:first-child{margin-left:14px;}
.ttl{position:absolute;left:0;right:0;text-align:center;font:500 14px Inter,"DejaVu Sans",sans-serif;color:#a9b1d6;letter-spacing:.02em;}
pre{margin:0;padding:%(PADY)dpx %(PADX)dpx;font-family:"JetBrains Mono","DejaVu Sans Mono",monospace;font-size:%(FS)spx;line-height:%(LH)spx;color:%(FG)s;
 font-variant-ligatures:none;font-feature-settings:"liga" 0,"calt" 0;white-space:pre;}
</style></head><body><div class="win"><div class="bar"><div class="dot" style="background:#ff5f57"></div><div class="dot" style="background:#febc2e"></div><div class="dot" style="background:#28c840"></div><div class="ttl">flowcli</div></div><pre>%(BODY)s</pre></div></body></html>"""


def main():
    src, dst, cols, rows = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    fs = float(sys.argv[5]) if len(sys.argv) > 5 else 15
    lh = float(sys.argv[6]) if len(sys.argv) > 6 else 20
    W = int(sys.argv[7]) if len(sys.argv) > 7 else 1040
    H = int(sys.argv[8]) if len(sys.argv) > 8 else 832
    text = open(src, encoding="utf-8", errors="replace").read()
    body, rgb = convert(text, cols, rows)
    barh = 32
    cw = fs * 0.6
    padx = int(round((W - cols * cw) / 2))
    pady = int(round((H - barh - rows * lh) / 2))
    page = PAGE % dict(W=W, H=H, BG=BG, FG=FG, BARH=barh, PADX=padx, PADY=pady, FS=fs, LH=lh, BODY=body)
    open(dst, "w", encoding="utf-8").write(page)
    sys.stderr.write("rgb sgr seen: %s\n" % sorted(rgb))


if __name__ == "__main__":
    main()
