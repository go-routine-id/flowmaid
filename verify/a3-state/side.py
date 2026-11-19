import sys
from PIL import Image, ImageDraw
d = sys.argv[1]
for n in sys.argv[2:]:
    ims = [Image.open(f"{d}/mm/{n}.mermaid.png"), Image.open(f"{d}/outbase/{n}.flowmaid.png"), Image.open(f"{d}/out/{n}.flowmaid.png")]
    ims = [i.convert("RGB") for i in ims]
    h = max(i.height for i in ims) + 30
    w = sum(i.width for i in ims) + 40
    c = Image.new("RGB", (w, h), "white")
    x = 0
    dr = ImageDraw.Draw(c)
    for t, i in zip(["mermaid", "base", "new"], ims):
        dr.text((x + 10, 5), t, fill="black")
        c.paste(i, (x, 30)); x += i.width + 20
        dr.line([(x-10, 0), (x-10, h)], fill="gray")
    c.save(f"{d}/side/{n}.png")
