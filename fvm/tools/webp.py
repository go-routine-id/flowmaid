# usage: python3 -I webp.py <max_w> <max_h> <out_dir> <png files...>
import sys, os
from PIL import Image
mw, mh, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
for f in sys.argv[4:]:
    im = Image.open(f).convert('RGB'); w, h = im.size
    k = min(1.0, mw / w, mh / h)
    if k < 1: im = im.resize((max(1, round(w * k)), max(1, round(h * k))), Image.LANCZOS)
    base = os.path.basename(f)[:-4]
    im.save(os.path.join(out, base + '.webp'), 'WEBP', quality=82, method=6)
    print(base, im.size[0], im.size[1])
