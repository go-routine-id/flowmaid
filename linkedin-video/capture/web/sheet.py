import sys
from PIL import Image, ImageDraw
d, out = sys.argv[1], sys.argv[2]
idx = [int(x) for x in sys.argv[3].split(',')]
cols = 3
ims = [Image.open(f'{d}/f{i:04d}.png').convert('RGB') for i in idx]
w, h = ims[0].size
s = 0.42
tw, th = int(w*s), int(h*s)
rows = (len(ims)+cols-1)//cols
sheet = Image.new('RGB', (cols*tw, rows*(th+18)), 'white')
dr = ImageDraw.Draw(sheet)
for k, (i, im) in enumerate(zip(idx, ims)):
    x, y = (k%cols)*tw, (k//cols)*(th+18)
    sheet.paste(im.resize((tw, th), Image.LANCZOS), (x, y+18))
    dr.text((x+4, y+3), f'f{i:04d}', fill='black')
sheet.save(out)
