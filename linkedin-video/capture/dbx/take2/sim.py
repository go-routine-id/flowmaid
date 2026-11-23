#!/usr/bin/env python3
"""Text-only simulation of dbx 0.5.10's ERD painter (draw_tables +
draw_edge_text in src/ui/screens/erd.rs) on flowmaid 0.25's scene
(geom.json, dumped by geom/), used only to PLAN framings. Every frame of the
take is checked on the real capture with check.py."""
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
G = json.load(open(os.path.join(HERE, "geom.json")))
PX_PER_COL, PX_PER_ROW, PAN_STEP_COLS, OVERSCROLL = 7.0, 22.0, 4.0, 60.0


def rround(x):  # Rust f64::round: half away from zero
    return math.floor(x + 0.5) if x >= 0 else -math.floor(-x + 0.5)


class View:
    def __init__(self):
        self.ox = 0.0
        self.oy = 0.0
        self.zoom = 1.0
        self.page_rows = 10  # set to canvas height - 1 by the caller

    def pc(self):
        return PX_PER_COL * self.zoom

    def pr(self):
        return PX_PER_ROW * self.zoom

    def key(self, k):
        sw, sh = G["width"], G["height"]
        if k == "-":
            self.zoom = max(self.zoom / 1.25, 0.25)
        elif k == "+":
            self.zoom = min(self.zoom * 1.25, 3.0)
        elif k == "l":
            self.ox = min(self.ox + self.pc() * PAN_STEP_COLS, sw)
        elif k == "h":
            self.ox = max(self.ox - self.pc() * PAN_STEP_COLS, -OVERSCROLL * self.pc())
        elif k == "j":
            self.oy = min(self.oy + self.pr(), sh)
        elif k == "k":
            self.oy = max(self.oy - self.pr(), -OVERSCROLL * self.pr())
        elif k in "DU":  # PageDown / PageUp: canvas height - 1 rows
            p = self.page_rows * self.pr()
            if k == "D":
                self.oy = min(self.oy + p, sh)
            else:
                self.oy = max(self.oy - p, -OVERSCROLL * self.pr())
        elif k == "0":
            self.ox = self.oy = 0.0
            self.zoom = 1.0
        return self


def clip(x, y, w, h, a):
    ax, ay, aw, ah = a
    l, t = max(x, ax), max(y, ay)
    r, b = min(x + w, ax + aw), min(y + h, ay + ah)
    if r <= l or b <= t:
        return None
    return (l, t, r - l, b - t)


def paint(view, area):
    """-> (boxes, tokens). boxes: dict(name, rect, full_rect); tokens:
    dict(text, drawn, row, col, kind)"""
    ax, ay, aw, ah = area
    pc, pr = view.pc(), view.pr()
    vx, vy = view.ox, view.oy
    boxes = []
    for n in G["nodes"]:
        x0 = rround((n["x"] - n["w"] / 2 - vx) / pc) + ax
        y0 = rround((n["y"] - n["h"] / 2 - vy) / pr) + ay
        w = int(max(rround(n["w"] / pc), 6))
        h = int(max(rround(n["h"] / pr), 3))
        rect = clip(x0, y0, w, h, area)
        if rect is None or rect[2] < 4 or rect[3] < 3:
            continue
        boxes.append({"name": n["name"], "rect": rect, "full": (x0, y0, w, h), "rows": n["rows"]})
    tokens = []

    def put(col, row, text, kind):
        if row < ay or row >= ay + ah or col < ax or col >= ax + aw:
            return
        mx = ax + aw - col
        tokens.append({"text": text, "drawn": text[:mx], "row": row, "col": col, "kind": kind})

    for e in G["edges"]:
        if e["label"]:
            text, (cx, cy), _w = e["label"]
            col = rround((cx - vx) / pc - len(text) / 2.0)
            row = rround((cy - vy) / pr)
            put(col, row, text, "label")
        b = e["bezier"]
        for (ep, cp, card) in ((b[0], b[1], e["cards"][0]), (b[3], b[2], e["cards"][1])):
            dx, dy = cp[0] - ep[0], cp[1] - ep[1]
            ln = max(math.sqrt(dx * dx + dy * dy), 1e-6)
            px, py = ep[0] + dx / ln * 22.0, ep[1] + dy / ln * 22.0
            col = rround((px - vx) / pc - len(card) / 2.0)
            row = rround((py - vy) / pr)
            put(col, row, card, "card")
    return boxes, tokens


def inner_lines(rows, H, Wd):
    out, full = [], []
    visible = min(len(rows), H)
    for r in range(H):
        line = [" "] * Wd
        ok = True
        if r < visible:
            if r == visible - 1 and len(rows) > H:
                s = "+%d more" % (len(rows) - H + 1)
                line[:min(len(s), Wd)] = s[:Wd]
                ok = len(s) <= Wd
            else:
                ty, name, keys = rows[r]
                left = "%s %s" % (ty, name)
                line[:min(len(left), Wd)] = left[:Wd]
                ok = len(left) <= Wd
                if keys:
                    kw = len(keys)
                    if Wd > kw:
                        line[Wd - kw:] = keys
                        ok = ok and len(left) + 1 + kw <= Wd
                    else:
                        ok = False
        out.append("".join(line))
        full.append(ok)
    return out, full


def render_text(boxes, tokens, cols, rows, area):
    """Pane-sized char grid (no braille, no chrome except the ERD frame)."""
    g = [[" "] * cols for _ in range(rows)]
    for b in boxes:
        x, y, w, h = b["rect"]
        g[y][x], g[y][x + w - 1] = "╭", "╮"
        g[y + h - 1][x], g[y + h - 1][x + w - 1] = "╰", "╯"
        for c in range(x + 1, x + w - 1):
            g[y][c] = g[y + h - 1][c] = "─"
        for r in range(y + 1, y + h - 1):
            g[r][x] = g[r][x + w - 1] = "│"
        t = b["name"]
        avail = w - 2
        if len(t) > avail:
            t = t[:avail]
        off = (avail - len(t)) // 2
        for i, ch in enumerate(t):
            g[y][x + 1 + off + i] = ch
        lines, _ = inner_lines(b["rows"], h - 2, w - 2)
        for k, l in enumerate(lines):
            for i, ch in enumerate(l):
                if ch != " ":
                    g[y + 1 + k][x + 1 + i] = ch
    for t in tokens:
        for i, ch in enumerate(t["drawn"]):
            g[t["row"]][t["col"] + i] = ch
    return ["".join(r) for r in g]


def defects(boxes, tokens, area):
    """Fast equivalent of check.py on the simulated paint."""
    ax, ay, aw, ah = area
    d, w = [], []
    occ = {}
    for i, b in enumerate(boxes):
        x, y, bw, bh = b["rect"]
        if len(b["name"]) > bw - 2:
            d.append("title of %s clipped" % b["name"])
        _, full = inner_lines(b["rows"], bh - 2, bw - 2)
        if not all(full):
            d.append("box %s: column text clipped" % b["name"])
        if b["rect"] != b["full"]:
            fx, fy, fw, fh = b["full"]
            if fx != x or fw != bw:
                w.append("box %s cut horizontally" % b["name"])
            else:
                w.append("box %s cut vertically" % b["name"])
        for r in range(y, y + bh):
            for c in range(x, x + bw):
                occ[(r, c)] = ("box", i)
    for i, t in enumerate(tokens):
        if t["drawn"] != t["text"]:
            w.append("token %s clipped at right edge" % t["text"])
        for k in range(len(t["drawn"])):
            key = (t["row"], t["col"] + k)
            if key in occ:
                o = occ[key]
                d.append("token %s over %s" % (t["text"], boxes[o[1]]["name"] if o[0] == "box" else tokens[o[1]]["text"]))
            occ[key] = ("tok", i)
    # tokens must not touch each other (they would read as one run)
    for i, t in enumerate(tokens):
        for side in (t["col"] - 1, t["col"] + len(t["drawn"])):
            o = occ.get((t["row"], side))
            if o and o[0] == "tok" and o[1] != i:
                d.append("token %s touches %s" % (t["text"], tokens[o[1]]["text"]))
    return d, w


def area_for(cols, rows, folded, tree_w=32):
    if folded:
        return (1, 4, cols - 2, rows - 8)
    return (tree_w + 1, 4, cols - tree_w - 2, rows - 8)


if __name__ == "__main__":
    cols, rows, nz = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    keys = sys.argv[4] if len(sys.argv) > 4 else ""
    v = View()
    for _ in range(nz):
        v.key("-")
    for k in keys:
        v.key(k)
    a = area_for(cols, rows, True)
    b, t = paint(v, a)
    for i, l in enumerate(render_text(b, t, cols, rows, a)):
        print("%3d %s" % (i, l.rstrip()))
    print(defects(b, t, a))
