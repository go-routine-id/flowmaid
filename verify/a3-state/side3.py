import sys, os
from PIL import Image, ImageDraw
d = sys.argv[1]
for n in sys.argv[2:]:
    paths = [(f"{d}/mm3/{n}.mermaid.png", "mermaid"), (f"{d}/mm/{n}.mermaid.png", "mermaid"), (f"{d}/out3/{n}.flowmaid.png", "flowmaid HEAD")]
    ims = []
    seen_mm = False
    for p, t in paths:
        if os.path.exists(p) and not (t == "mermaid" and seen_mm):
            ims.append((Image.open(p).convert("RGB"), t))
            if t == "mermaid": seen_mm = True
    h = max(i.height for i, _ in ims) + 30
    w = sum(i.width for i, _ in ims) + 20 * len(ims)
    c = Image.new("RGB", (w, h), "white")
    x = 0
    dr = ImageDraw.Draw(c)
    for i, t in ims:
        dr.text((x + 10, 5), t, fill="black")
        c.paste(i, (x, 30)); x += i.width + 20
        dr.line([(x-10, 0), (x-10, h)], fill="gray")
    c.save(f"{d}/side3/{n}.png")
