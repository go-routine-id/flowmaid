import re,sys,subprocess,os
from PIL import Image
a,b,out=sys.argv[1:4]
ca=set(re.findall(r'<circle cx="([\d.]+)" cy="([\d.]+)"',open(a).read()))
cb=re.findall(r'<circle cx="([\d.]+)" cy="([\d.]+)"',open(b).read())
new=[(float(x),float(y)) for x,y in cb if (x,y) not in ca]
if not new: print('none'); sys.exit()
x0=min(p[0] for p in new)-40; x1=max(p[0] for p in new)+40; y0=min(p[1] for p in new)-40; y1=max(p[1] for p in new)+40
S=4
env=dict(os.environ,SCALE=str(S))
tiles=[]
for f in (a,b):
    png=f+'.png'
    subprocess.run(['node','/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/sqf/tools/png.js',png,f],env=env,check=True)
    im=Image.open(png); ox,oy=6*S,22*S
    tiles.append(im.crop((int(ox+x0*S),int(oy+y0*S),int(ox+x1*S),int(oy+y1*S))))
w=sum(t.size[0] for t in tiles)+20; h=max(t.size[1] for t in tiles)
c=Image.new('RGB',(w,h),'white'); x=0
for t in tiles: c.paste(t,(x,0)); x+=t.size[0]+20
c.save(out); print(len(new),'new holes; box',x0,y0,x1,y1)
