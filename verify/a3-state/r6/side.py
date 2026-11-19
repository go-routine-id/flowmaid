import sys, os
from PIL import Image, ImageDraw
d = sys.argv[1]
for n in sys.argv[2:]:
    cand = [("mermaid", f"{d}/mm/{n}.mermaid.png"), ("911eb69", f"{d}/fmprev/{n}.flowmaid.png"), ("now", f"{d}/fm/{n}.flowmaid.png")]
    cand = [(t, p) for t, p in cand if os.path.exists(p)]
    ims = [(t, Image.open(p).convert("RGB")) for t, p in cand]
    h = max(i.height for _, i in ims) + 30
    w = sum(i.width for _, i in ims) + 20 * len(ims)
    c = Image.new("RGB", (w, h), "white")
    x = 0
    dr = ImageDraw.Draw(c)
    for t, i in ims:
        dr.text((x + 10, 5), t, fill="black")
        c.paste(i, (x, 30)); x += i.width + 20
        dr.line([(x-10, 0), (x-10, h)], fill="gray")
    c.save(f"{d}/side/{n}.png")
