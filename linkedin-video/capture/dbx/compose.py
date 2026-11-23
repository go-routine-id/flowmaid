#!/usr/bin/env python3
"""Lay the captured dbx states out on a 30 fps timeline as hard-linked frames."""
import json, os, shutil
from PIL import Image

W = os.path.dirname(os.path.abspath(__file__))
OUT = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/clips/dbx"
TIMELINE = [
    ("02-tables", 30),        # connected, table list of the SQLite db
    ("04-erd", 21),           # 'g' -> ERD tab, laid out by flowmaid
    ("05-erd-full", 33),      # Ctrl+B folds the tree: whole schema
    ("07-zoom", 15),          # '-' zoom in (0.80x)
] + [("08-pan-%d" % i, 3) for i in range(1, 9)] + [  # 'l' x8 pan right
    ("09-reset", 12),         # '0' reset view
    ("10-sel1", 6),           # '.' select customers
    ("10-sel2", 6),           # '.' select order_items
    ("11-sel-settled", 45),   # '.' select orders (hold)
]
POSTER = "11-sel-settled"

shutil.rmtree(OUT, ignore_errors=True)
os.makedirs(OUT)
size = None
n = 0
for name, count in TIMELINE:
    src = os.path.join(W, "png", name + ".png")
    s = Image.open(src).size
    assert size in (None, s), (name, s, size)
    size = s
    for _ in range(count):
        n += 1
        os.link(src, os.path.join(OUT, "f%04d.png" % n))
shutil.copyfile(os.path.join(W, "png", POSTER + ".png"), os.path.join(OUT, "poster.png"))
json.dump({"width": size[0], "height": size[1], "fps": 30, "frames": n}, open(os.path.join(OUT, "manifest.json"), "w"))
print(size, n, n / 30.0)
