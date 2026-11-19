import sys, os
from PIL import Image, ImageDraw
d = sys.argv[1]
for n in sys.argv[2:]:
    mm = f"{d}/mm3/{n}.mermaid.png"
    if not os.path.exists(mm): mm = f"{d}/mm/{n}.mermaid.png"
    paths = [(mm, "mermaid"), (f"{d}/outmb3/{n}.flowmaid.png", "merge-base"), (f"{d}/out3/{n}.flowmaid.png", "HEAD")]
    ims = [(Image.open(p).convert("RGB"), t) for p, t in paths if os.path.exists(p)]
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
