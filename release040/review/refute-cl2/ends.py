import re,sys
# self-loops in a flowmaid class SVG: a single cubic "M x y C .., .., x y" whose two ends share x or y.
# report, per loop, how far each end lies outside the nearest class box (0 = on the box).
def run(path):
    s=open(path).read()
    rects=[tuple(map(float,m)) for m in re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="4" fill="none"',s)]
    res=[]
    for m in re.finditer(r'<path d="M ([\d.]+) ([\d.]+) C ([\d.]+) ([\d.]+), ([\d.]+) ([\d.]+), ([\d.]+) ([\d.]+)" fill="none"',s):
        v=list(map(float,m.groups())); a=(v[0],v[1]); b=(v[6],v[7])
        if not (abs(a[0]-b[0])<1e-6 or abs(a[1]-b[1])<1e-6): continue
        # control points bulge the same way on a loop
        best=None
        for (x,y,w,h) in rects:
            def d(p):
                dx=max(x-p[0],0,p[0]-(x+w)); dy=max(y-p[1],0,p[1]-(y+h)); return (dx*dx+dy*dy)**.5
            t=(d(a)+d(b),max(d(a),d(b)),(x,y,w,h))
            if best is None or t<best: best=t
        if best and best[1]<80:
            # a straight vertical/horizontal relation between two boxes also has ends sharing x; a loop bulges: control points off the line
            if abs(a[0]-b[0])<1e-6 and abs(v[2]-a[0])<1e-6 and abs(v[4]-a[0])<1e-6: continue
            if abs(a[1]-b[1])<1e-6 and abs(v[3]-a[1])<1e-6 and abs(v[5]-a[1])<1e-6: continue
            res.append((round(best[1],1),a,b,best[2]))
    return res
for p in sys.argv[1:]:
    r=run(p)
    print(p.split('/')[-1], 'loops',len(r),'max end off box',max([x[0] for x in r],default=0), [x[0] for x in r])
