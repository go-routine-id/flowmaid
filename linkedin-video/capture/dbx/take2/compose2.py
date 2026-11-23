#!/usr/bin/env python3
"""Lay the captured take-2 dbx states out on a 30 fps timeline (hard links)."""
import json, os, shutil, sys
from PIL import Image

W2 = os.path.dirname(os.path.abspath(__file__))
OUT = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/clips/dbx2"
TIMELINE = [
    ("01-explorer", 18, "explorer: connected, tables of main"),
    ("02-erd", 8, "g: ERD generated (default zoom, unfolded)"),
    ("03-drag-00", 12, "mouse drag on the canvas: view moves right"),
] + [("04-drag-%02d" % i, 1, "mouse drag: view moves down" if i == 1 else None) for i in range(1, 14)] + [
    ("05-drag-done", 6, None),
    ("06-fold", 15, "Ctrl+B: tree folded, ERD full width"),
    ("07-zoom", 18, "'-': zoom 1.00x -> 0.80x"),
    ("08-k1", 9, "k: pan up"),
    ("08-k2", 9, "k: pan up"),
    ("08-k3", 10, "k: pan up"),
    ("09-h1", 10, "h: pan left"),
    ("09-h2", 12, "h: pan left"),
    ("10-sel1", 4, "'.': select next (customers, off-screen)"),
    ("10-sel2", 4, "'.': select next (order_items, off-screen)"),
    ("10-sel3", 8, "'.': select next (orders, highlighted)"),
    ("11-hold", 32, "hold"),
]
POSTER = "11-hold"

if __name__ == "__main__":
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(OUT)
    size, n, events, frames = None, 0, [], []
    for name, count, ev in TIMELINE:
        src = os.path.join(W2, "png", name + ".png")
        s = Image.open(src).size
        assert size in (None, s), (name, s, size)
        size = s
        if ev:
            events.append((n + 1, name, ev))
        for _ in range(count):
            n += 1
            os.link(src, os.path.join(OUT, "f%04d.png" % n))
            frames.append(name)
    shutil.copyfile(os.path.join(W2, "png", POSTER + ".png"), os.path.join(OUT, "poster.png"))
    json.dump({"width": size[0], "height": size[1], "fps": 30, "frames": n},
              open(os.path.join(OUT, "manifest.json"), "w"))
    json.dump({"events": events, "frames": frames}, open(os.path.join(W2, "timeline.json"), "w"), indent=0)
    print(size, n, n / 30.0)
    for e in events:
        print(e)
