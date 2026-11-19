# overlap.py <raw> <edge> [stage]: segments of other edges parallel to <edge>'s segments within 6.5 px, overlapping > 2 px
import sys,re
raw,me=sys.argv[1],sys.argv[2]; stage=sys.argv[3] if len(sys.argv)>3 else 'crate::ortho::orthogonalize'
P={}
for l in open(raw):
    m=re.match(r'^(\S+) \[([^\]]+)\] bez=\(([-\d.]+),([-\d.]+)\)\.\.\(([-\d.]+),([-\d.]+)\) n=(\d+) ?(.*)$',l.rstrip())
    if m and m.group(2)==stage:
        pts=[tuple(map(float,p.split(','))) for p in re.findall(r'\(([-\d.]+,[-\d.]+)\)',m.group(8))]
        if not pts: pts=[(float(m.group(3)),float(m.group(4))),(float(m.group(5)),float(m.group(6)))]
        P[m.group(1)]=pts
def segs(p):
    for a,b in zip(p,p[1:]):
        if abs(a[0]-b[0])<1e-6: yield ('v',a[0],min(a[1],b[1]),max(a[1],b[1]))
        elif abs(a[1]-b[1])<1e-6: yield ('h',a[1],min(a[0],b[0]),max(a[0],b[0]))
for o,p in P.items():
    if o==me: continue
    for s in segs(P[me]):
        for t in segs(p):
            if s[0]==t[0] and abs(s[1]-t[1])<=6.5:
                ov=min(s[3],t[3])-max(s[2],t[2])
                if ov>2: print(f'{me} {s} ~ {o} {t} overlap {ov:.1f} off {abs(s[1]-t[1]):.1f}')
