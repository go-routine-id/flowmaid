import sys, os, re, glob
from PIL import Image
import numpy as np
# usage: margins.py rundir [names...]
d = sys.argv[1]
names = sys.argv[2:] or sorted(os.path.basename(p)[:-4] for p in glob.glob(d + '/*.svg'))
print(f"{'name':16} {'W':>7} {'H':>7} {'left':>6} {'right':>6} {'top':>6} {'bot':>6} {'|l-r|':>6} {'|t-b|':>6}")
for n in names:
    svg = open(f"{d}/{n}.svg").read()
    m = re.search(r'viewBox="([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+)"', svg)
    W, H = float(m.group(3)), float(m.group(4))
    im = np.asarray(Image.open(f"{d}/{n}.png").convert('RGB')).astype(int)
    pad = 12  # 8px css padding * 1.5
    im = im[pad:-pad, pad:-pad]
    s = im.shape[1] / W
    ink = (im.min(axis=2) < 240)
    ys, xs = np.nonzero(ink)
    if len(xs) == 0:
        continue
    l, r = xs.min() / s, (im.shape[1] - 1 - xs.max()) / s
    t, b = ys.min() / s, (im.shape[0] - 1 - ys.max()) / s
    print(f"{n:16} {W:7.1f} {H:7.1f} {l:6.1f} {r:6.1f} {t:6.1f} {b:6.1f} {abs(l-r):6.1f} {abs(t-b):6.1f}")
