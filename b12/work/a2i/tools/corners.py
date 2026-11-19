# corners.py <svg> <struct.json> <edge-name...>: the corners (axis turns) of edges, with the frame each lies in
import json,re,sys
svg=open(sys.argv[1]).read(); st=json.load(open(sys.argv[2]))
M='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2i/tools/bm-gapedge'
import subprocess
j=json.loads(subprocess.run([M,'--struct',sys.argv[2],'--json',sys.argv[1]],capture_output=True,text=True).stdout)
m=j[0] if isinstance(j,list) else j
mod=m.get('model',m)
frames=mod.get('frames',[]); edges=mod.get('edges',[])
def inframes(p):
    out=[]
    for f in frames:
        b=f['box']; 
        if b['x']<=p[0]<=b['x']+b['w'] and b['y']<=p[1]<=b['y']+b['h']: out.append(f.get('id'))
    return '/'.join(out) or '-'
for e in edges:
    if e.get('name') not in sys.argv[3:]: continue
    pts=[]
    for s in e['segs']: 
        a=(s[0],s[1]) if isinstance(s[0],(int,float)) else tuple(s[0]); b=(s[2],s[3]) if isinstance(s[0],(int,float)) else tuple(s[1])
        if not pts: pts.append(a)
        pts.append(b)
    turns=[]
    for i in range(1,len(pts)-1):
        d1=(pts[i][0]-pts[i-1][0],pts[i][1]-pts[i-1][1]); d2=(pts[i+1][0]-pts[i][0],pts[i+1][1]-pts[i][1])
        h1=abs(d1[0])>abs(d1[1]); h2=abs(d2[0])>abs(d2[1])
        if (abs(d1[0])+abs(d1[1]))>0.5 and (abs(d2[0])+abs(d2[1]))>0.5 and h1!=h2: turns.append(pts[i])
    print(e['name'], len(turns), 'turns:', ' '.join(f"({p[0]:.0f},{p[1]:.0f})[{inframes(p)}]" for p in turns))
