# crop a mermaid render (made by mmr.js: deviceScaleFactor 1.5, 8 px page padding) in SVG px coordinates
# usage: crop.py in.png out.png x0 y0 x1 y1 [maxwidth]
import sys
from PIL import Image
i, o, x0, y0, x1, y1 = sys.argv[1], sys.argv[2], *map(float, sys.argv[3:7])
mw = int(sys.argv[7]) if len(sys.argv) > 7 else 1800
im = Image.open(i).convert('RGB'); k = 1.5; pad = 8 * k
box = (int(x0 * k + pad), int(y0 * k + pad), int(min(im.width, x1 * k + pad)), int(min(im.height, y1 * k + pad)))
c = im.crop(box)
if c.width > mw:
    c = c.resize((mw, int(c.height * mw / c.width)), Image.LANCZOS)
c.save(o); print(o, c.size)
