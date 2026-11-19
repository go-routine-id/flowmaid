import sys
from PIL import Image
# usage: crop.py name x0 y0 x1 y1 out   (fractions of image size) -> pre|head side by side
V=__import__("os").environ.get("VIS","/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2c-squeeze/vis/")
name=sys.argv[1]; f=[float(x) for x in sys.argv[2:6]]; out=sys.argv[6]
ims=[]
for w in ("pre","head"):
    im=Image.open(V+w+"/"+name+".png"); W,H=im.size
    ims.append(im.crop((int(f[0]*W),int(f[1]*H),int(f[2]*W),int(f[3]*H))))
w=sum(i.size[0] for i in ims)+20; h=max(i.size[1] for i in ims)
o=Image.new("RGB",(w,h),"white"); x=0
for i in ims:
    o.paste(i,(x,0)); x+=i.size[0]+20
o.save(out)
