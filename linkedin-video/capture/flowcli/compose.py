#!/usr/bin/env python3
"""Compose two tmux pane captures (ANSI, from capture-pane -e -p) side by side
into one HTML terminal window.

usage: compose.py spec.json   (list of states; writes one .html per state)
spec item: {"left": path, "right": path, "out": path,
            "cursor": [x, y] | null   (left-pane cursor cell, 0-based),
            "pointer": [col, row] | null (right-pane cell, 0-based)}
"""
import html, json, sys
sys.path.insert(0, sys.path[0])
from ansi2html import convert, BG, FG

LW, RW, ROWS = 49, 60, 38
FS, LH = 15.0, 20.0
CW = FS * 0.6
BARH = 30
PADX, PADY = 16, 10
W = int(round(PADX * 2 + (LW + 1 + RW) * CW))
H = BARH + PADY * 2 + int(ROWS * LH)
W += W % 2
H += H % 2

CSS = f"""
html,body{{margin:0;padding:0;background:{BG};}}
.win{{position:absolute;left:0;top:0;width:{W}px;height:{H}px;background:{BG};overflow:hidden;}}
.bar{{height:{BARH}px;background:#1c1f2e;position:relative;border-bottom:1px solid #262a3d;box-sizing:border-box;}}
.dot{{position:absolute;top:{(BARH-12)//2}px;width:12px;height:12px;border-radius:50%;}}
.ttl{{position:absolute;left:0;right:0;top:0;text-align:center;font:500 14px Inter,"DejaVu Sans",sans-serif;line-height:{BARH}px;color:#a9b1d6;letter-spacing:.03em;}}
pre{{position:absolute;left:{PADX}px;top:{BARH+PADY}px;margin:0;padding:0;font-family:"JetBrains Mono","DejaVu Sans Mono",monospace;font-size:{FS}px;line-height:{LH}px;color:{FG};
 font-variant-ligatures:none;font-feature-settings:"liga" 0,"calt" 0;white-space:pre;}}
.ptr{{position:absolute;width:18px;height:26px;}}
"""

def rows_html(path, cols, cursor=None):
    text = open(path, encoding="utf-8", errors="replace").read()
    body, _ = convert(text, cols, ROWS, cursor=cursor)
    return body.split("\n")

def page(left, right, cursor, pointer):
    L = rows_html(left, LW, cursor)
    R = rows_html(right, RW)
    div = '<span style="color:#2c3150">│</span>'
    body = "\n".join(l + div + r for l, r in zip(L, R))
    ptr = ""
    if pointer:
        c, r = pointer
        x = PADX + (LW + 1 + c + 0.5) * CW
        y = BARH + PADY + (r + 0.5) * LH
        ptr = (f'<svg class="ptr" style="left:{x-2:.1f}px;top:{y-2:.1f}px" viewBox="0 0 18 26">'
               '<path d="M2 2 L2 21 L7 16.5 L10.5 24 L14 22.5 L10.5 15 L17 15 Z" '
               'fill="#ffffff" stroke="#111" stroke-width="1.4" stroke-linejoin="round"/></svg>')
    dots = "".join(f'<div class="dot" style="left:{14+i*20}px;background:{c}"></div>'
                   for i, c in enumerate(["#ff5f57", "#febc2e", "#28c840"]))
    return (f'<!doctype html><html><head><meta charset="utf-8"><style>{CSS}</style></head><body>'
            f'<div class="win"><div class="bar">{dots}<div class="ttl">flowcli</div></div>'
            f'<pre>{body}</pre>{ptr}</div></body></html>')

if __name__ == "__main__":
    spec = json.load(open(sys.argv[1]))
    for s in spec:
        open(s["out"], "w", encoding="utf-8").write(
            page(s["left"], s["right"], s.get("cursor"), s.get("pointer")))
    print(json.dumps({"W": W, "H": H, "n": len(spec)}))
