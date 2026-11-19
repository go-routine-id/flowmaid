import sys
from PIL import Image
Image.MAX_IMAGE_PIXELS=None
fn,x0,y0,x1,y1,out=sys.argv[1:7]
im=Image.open(fn)
sc=im.width/2259.0 if 'flowmaid' in fn else float(sys.argv[7]) if len(sys.argv)>7 else 1
box=tuple(int(float(v)*sc) for v in (x0,y0,x1,y1))
im.crop(box).save(out)
print(im.size, sc, box)
