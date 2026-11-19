import sys
from PIL import Image, ImageDraw
# usage: pair.py pre.png cur.png out.png [vertical] [crop x0 y0 x1 y1 (fractions)]
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
out = sys.argv[3]
vert = len(sys.argv) > 4 and sys.argv[4] == "v"
if len(sys.argv) > 5:
    f = [float(x) for x in sys.argv[5:9]]
    def cr(im):
        w, h = im.size
        return im.crop((int(f[0]*w), int(f[1]*h), int(f[2]*w), int(f[3]*h)))
    a, b = cr(a), cr(b)
if vert:
    W = max(a.width, b.width); H = a.height + b.height + 30
    im = Image.new("RGB", (W, H), "white"); im.paste(a, (0, 15)); im.paste(b, (0, a.height + 30))
    d = ImageDraw.Draw(im); d.text((2, 0), "PRE", fill="red"); d.text((2, a.height + 15), "NOW", fill="red")
else:
    W = a.width + b.width + 20; H = max(a.height, b.height) + 15
    im = Image.new("RGB", (W, H), "white"); im.paste(a, (0, 15)); im.paste(b, (a.width + 20, 15))
    d = ImageDraw.Draw(im); d.text((2, 0), "PRE", fill="red"); d.text((a.width + 22, 0), "NOW", fill="red")
im.save(out)
print(im.size)
