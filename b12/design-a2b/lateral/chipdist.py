# chipdist.py <probe.raw> [stage]: per traced edge, at the given stage (default ortho), the title heads its path
# comes within 8 px of: the least distance and where (x,y), plus the node boxes of its ends.
import sys,re,math
raw=sys.argv[1]; stage=sys.argv[2] if len(sys.argv)>2 else 'crate::ortho::orthogonalize'
nodes={}; cl=[]; paths={}
for l in open(raw):
    l=l.rstrip()
    m=re.match(r'NODE (\S+) .*box=\(([-\d.]+),([-\d.]+)\)-\(([-\d.]+),([-\d.]+)\)',l)
    if m: nodes[m.group(1)]=tuple(map(float,m.groups()[1:])); continue
    m=re.match(r"CLUSTER (\S+) '(.*)' box=\(([-\d.]+),([-\d.]+)\)-\(([-\d.]+),([-\d.]+)\) chip=\(([-\d.]+),([-\d.]+)\)-\(([-\d.]+),([-\d.]+)\)",l)
    if m:
        g=m.groups(); fb=tuple(map(float,g[2:6])); cb=tuple(map(float,g[6:10]))
        cl.append((g[0],fb,(cb[0],fb[1],cb[2],cb[3]))); continue
    m=re.match(r'^(\S+) \[([^\]]+)\] bez=\(([-\d.]+),([-\d.]+)\)\.\.\(([-\d.]+),([-\d.]+)\) n=(\d+) ?(.*)$',l)
    if m and m.group(2)==stage:
        e=m.group(1); pts=[tuple(map(float,p.split(','))) for p in re.findall(r'\(([-\d.]+,[-\d.]+)\)',m.group(8))]
        if not pts: pts=[(float(m.group(3)),float(m.group(4))),(float(m.group(5)),float(m.group(6)))]
        paths[e]=pts
def segrect(a,b,r):
    # least distance between segment ab (axis-aligned or not) and rect r, sampled
    best=(1e9,None); n=max(2,int(math.hypot(b[0]-a[0],b[1]-a[1])/0.25))
    for i in range(n+1):
        t=i/n; x=a[0]+(b[0]-a[0])*t; y=a[1]+(b[1]-a[1])*t
        dx=max(r[0]-x,0,x-r[2]); dy=max(r[1]-y,0,y-r[3]); d=math.hypot(dx,dy)
        if d<best[0]: best=(d,(round(x,1),round(y,1)))
    return best
for e,pts in paths.items():
    f,t=e.split('->')
    out=[]
    for cid,fb,head in cl:
        d=(1e9,None)
        for a,b in zip(pts,pts[1:]):
            x=segrect(a,b,head)
            if x[0]<d[0]: d=x
        if d[0]<=8.0: out.append(f'{cid} head=({head[0]:.1f},{head[1]:.1f})-({head[2]:.1f},{head[3]:.1f}) dist={d[0]:.2f} at {d[1]}')
    print(f'{e}: from {nodes.get(f)} to {nodes.get(t)}')
    for o in out: print('   ',o)
