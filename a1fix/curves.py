import re,sys,glob
def curvy(d):
    if 'L' in d: return False
    nums=list(map(float,re.findall(r'-?\d+\.?\d*',d)))
    if len(nums)!=8: return False
    (x0,y0,x1,y1,x2,y2,x3,y3)=nums
    cross=lambda ax,ay: abs((x3-x0)*(ay-y0)-(y3-y0)*(ax-x0))
    return cross(x1,y1)>50 or cross(x2,y2)>50
for d in sys.argv[1:]:
    tot=0; out=[]
    for f in sorted(glob.glob(d+'/*.svg')):
        s=open(f).read()
        paths=re.findall(r'<path d="([^"]*)" fill="none" stroke="#44507a" stroke-width="1.6"',s)
        n=sum(curvy(p) for p in paths)
        if n: out.append((f.split('/')[-1].replace('.flowmaid.svg',''),n)); tot+=n
    print(d.split('/')[-1], tot, out)
