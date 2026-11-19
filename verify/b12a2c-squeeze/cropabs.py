import sys
from PIL import Image
V=sys.argv[1]; name=sys.argv[2]; x0,y0,x1,y1=[int(v) for v in sys.argv[3:7]]; out=sys.argv[7]
ims=[Image.open(f"{V}/{w}/{name}.png").crop((x0,y0,x1,y1)) for w in ("pre","head")]
o=Image.new("RGB",(ims[0].size[0]*2+20,ims[0].size[1]),"white"); o.paste(ims[0],(0,0)); o.paste(ims[1],(ims[0].size[0]+20,0)); o.save(out)
