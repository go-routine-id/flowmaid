import sys
from PIL import Image
a,b,out=sys.argv[1:4]
A=Image.open(a); B=Image.open(b)
W=A.width+B.width+20; H=max(A.height,B.height)
im=Image.new('RGB',(W,H),'white'); im.paste(A,(0,0)); im.paste(B,(A.width+20,0))
for x in range(A.width+8, A.width+12):
    for y in range(H): im.putpixel((x,y),(200,0,0))
im.save(out)
