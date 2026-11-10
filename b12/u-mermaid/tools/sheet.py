# contact sheet: python3 sheet.py out.png cellW label1=img1 label2=img2 ... (--cols N)
import sys
from PIL import Image, ImageDraw
out = sys.argv[1]; cellw = int(sys.argv[2]); items = []; cols = 2
for a in sys.argv[3:]:
    if a.startswith('--cols='): cols = int(a[7:]); continue
    k, v = a.split('=', 1); items.append((k, v))
cells = []
for k, v in items:
    im = Image.open(v).convert('RGB')
    s = min(1.0, cellw / im.width)
    im = im.resize((max(1, int(im.width * s)), max(1, int(im.height * s))), Image.LANCZOS)
    c = Image.new('RGB', (cellw, im.height + 22), 'white'); c.paste(im, (0, 22))
    ImageDraw.Draw(c).text((4, 4), k, fill=(200, 0, 0))
    cells.append(c)
rows = [cells[i:i + cols] for i in range(0, len(cells), cols)]
H = sum(max(c.height for c in r) + 6 for r in rows)
sheet = Image.new('RGB', (cellw * cols + 6 * (cols - 1), H), (230, 230, 230)); y = 0
for r in rows:
    x = 0
    for c in r: sheet.paste(c, (x, y)); x += cellw + 6
    y += max(c.height for c in r) + 6
sheet.save(out); print(out, sheet.size)
