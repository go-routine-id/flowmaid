import sys
from PIL import Image
a,b,out=sys.argv[1:4]
A=Image.open(a).convert('RGB'); B=Image.open(b).convert('RGB')
w=max(A.width,B.width); h=A.height+B.height+20
C=Image.new('RGB',(w,h),'white'); C.paste(A,(0,0)); C.paste(B,(0,A.height+20))
for x in range(w): C.putpixel((x,A.height+10),(200,0,0))
C.save(out)
