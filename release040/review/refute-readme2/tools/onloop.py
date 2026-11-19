import re,sys
def cubic(p0,p1,p2,p3,t):
    u=1-t
    return (u**3*p0[0]+3*u*u*t*p1[0]+3*u*t*t*p2[0]+t**3*p3[0], u**3*p0[1]+3*u*u*t*p1[1]+3*u*t*t*p2[1]+t**3*p3[1])
for f in sys.argv[1:]:
    s=open(f).read()
    # label rects: rect with rx=4 fill #ffffff stroke #d5d9ec followed by text
    labels=[(float(a),float(b),float(c),float(d),t) for a,b,c,d,t in re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="4" fill="#ffffff" stroke="#d5d9ec"/>\s*<text[^>]*>([^<]*)</text>',s)]
    # self loops: single-cubic path M a b C ... ending on same x or same y face (start/end close)
    loops=[]
    for m in re.finditer(r'<path d="M ([\d.-]+) ([\d.-]+) C ([\d.-]+) ([\d.-]+), ([\d.-]+) ([\d.-]+), ([\d.-]+) ([\d.-]+)" fill="none"',s):
        v=list(map(float,m.groups())); p=[(v[0],v[1]),(v[2],v[3]),(v[4],v[5]),(v[6],v[7])]
        # loop: control points both on the same side, start and end on the same face
        if (abs(p[0][0]-p[3][0])<0.01 and abs(p[1][0]-p[2][0])<0.01 and abs(p[1][0]-p[0][0])>20) or (abs(p[0][1]-p[3][1])<0.01 and abs(p[1][1]-p[2][1])<0.01 and abs(p[1][1]-p[0][1])>20):
            loops.append(p)
    hits=0
    out=[]
    for (x,y,w,h,t) in labels:
        for i,p in enumerate(loops):
            n=0; deep=0
            for k in range(2001):
                q=cubic(*p,k/2000)
                if x<q[0]<x+w and y<q[1]<y+h:
                    n+=1
                    d=min(q[0]-x,x+w-q[0],q[1]-y,y+h-q[1])
                    deep=max(deep,d)
            if n:
                hits+=1; out.append(f"   label '{t}' rect x {x:.1f}..{x+w:.1f} y {y:.1f}..{y+h:.1f} is crossed by loop#{i} {p} (deepest {deep:.1f}px inside)")
    print(f"{f}: {len(labels)} labels, {len(loops)} loops, {hits} label/loop overlaps")
    print("\n".join(out))
