import re,math,sys
s=open(sys.argv[1]).read()
nodes=[]
for m in re.finditer(r'<(rect|polygon|ellipse)([^>]*)/>\n<text x="([\d.]+)" y="([\d.]+)"[^>]*><tspan[^>]*>([^<]*)</tspan>',s):
    tag,attrs,x,y,t=m.groups(); x=float(x);y=float(y)
    if tag=='rect':
        g=dict(re.findall(r'(\w+)="([\d.]+)"',attrs)); w=float(g['width']);h=float(g['height'])
    elif tag=='ellipse':
        g=dict(re.findall(r'(\w+)="([\d.]+)"',attrs)); w=2*float(g['rx']);h=2*float(g['ry'])
    else:
        pts=[tuple(map(float,p.split(','))) for p in re.search(r'points="([^"]*)"',attrs).group(1).split()]
        w=max(p[0] for p in pts)-min(p[0] for p in pts); h=max(p[1] for p in pts)-min(p[1] for p in pts)
    nodes.append((t,x,y,w,h,tag))
def ex(n,dx,dy):
    t,x,y,w,h,tag=n; L=math.hypot(dx,dy)
    if tag=='ellipse':
        ux,uy=dx/L,dy/L; return 1/math.sqrt((ux/(w/2))**2+(uy/(h/2))**2)
    ux,uy=abs(dx)/L,abs(dy)/L; return min((w/2)/ux if ux>1e-9 else 1e9,(h/2)/uy if uy>1e-9 else 1e9)
byc={(round(n[1],1),round(n[2],1)):n for n in nodes}
out=[]
for e in re.findall(r'<path d="M ([\d.]+) ([\d.]+) C [\d. ]+ ([\d.]+) ([\d.]+)"',s):
    ax,ay,bx,by=map(float,e); a=byc[(ax,ay)]; b=byc[(bx,by)]
    d=math.hypot(bx-ax,by-ay); out.append((a[0],b[0],round(d-ex(a,bx-ax,by-ay)-ex(b,ax-bx,ay-by),1)))
print(out)
