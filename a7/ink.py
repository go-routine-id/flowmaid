import sys
from PIL import Image
for f in sys.argv[1:]:
    im=Image.open(f).convert('RGB')
    w,h=im.size
    # crop the 8px*1.5 padding frame
    px=im.load(); pad=12
    tot=0; ink=0
    for y in range(pad,h-pad):
        for x in range(pad,w-pad):
            r,g,b=px[x,y]; tot+=1
            if r<245 or g<245 or b<245: ink+=1
    print(f.split('/')[-1], w, h, "ink %.3f"%(ink/tot))
