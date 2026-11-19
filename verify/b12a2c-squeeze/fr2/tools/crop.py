import sys
from PIL import Image
im=Image.open(sys.argv[1]); x0,y0,x1,y1=[int(float(v)) for v in sys.argv[2:6]]
im.crop((x0,y0,x1,y1)).save(sys.argv[6])
