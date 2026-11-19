import sys
from PIL import Image, ImageDraw
V=sys.argv[1]
for n in sys.argv[2:]:
    ims=[Image.open(f"{V}/r-mm/{n}.mermaid.png").convert("RGB"),Image.open(f"{V}/r-pre/{n}.flowmaid.png").convert("RGB"),Image.open(f"{V}/r-now/{n}.flowmaid.png").convert("RGB")]
    h=max(i.height for i in ims)+30; w=sum(i.width for i in ims)+40
    out=Image.new("RGB",(w,h),"white"); d=ImageDraw.Draw(out); x=0
    for lab,i in zip(["mermaid","pre","now"],ims):
        d.text((x+5,5),lab,fill="black"); out.paste(i,(x,30)); x+=i.width+20
        d.line([(x-10,0),(x-10,h)],fill="gray")
    out.save(f"{V}/{n}.tri.png")
