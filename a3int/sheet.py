import sys
from PIL import Image
d, out = sys.argv[1], sys.argv[2]
names = sys.argv[3:]
ims = [Image.open(f"{d}/{n}.flowmaid.png") for n in names]
W = sum(i.width for i in ims) + 10 * len(ims); H = max(i.height for i in ims)
o = Image.new('RGB', (W, H), 'white'); x = 0
for i in ims:
    o.paste(i, (x, 0)); x += i.width + 10
o.save(out); print(o.size)
