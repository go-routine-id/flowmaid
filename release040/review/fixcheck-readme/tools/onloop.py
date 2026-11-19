import re, sys
def bez(p0,p1,p2,p3,t):
    u=1-t
    return (u**3*p0[0]+3*u*u*t*p1[0]+3*u*t*t*p2[0]+t**3*p3[0], u**3*p0[1]+3*u*u*t*p1[1]+3*u*t*t*p2[1]+t**3*p3[1])
for f in sys.argv[1:]:
    s=open(f).read()
    loops=[]
    for m in re.finditer(r'<path d="M ([-\d.]+) ([-\d.]+) C ([-\d.]+) ([-\d.]+), ([-\d.]+) ([-\d.]+), ([-\d.]+) ([-\d.]+)" fill="none"', s):
        v=list(map(float,m.groups()))
        p0,p1,p2,p3=(v[0],v[1]),(v[2],v[3]),(v[4],v[5]),(v[6],v[7])
        # a self loop: start and end on same x or same y, close together, control points bulging
        if (abs(p0[0]-p3[0])<0.1 and abs(p0[1]-p3[1])<60 and abs(p1[0]-p0[0])>20) or (abs(p0[1]-p3[1])<0.1 and abs(p0[0]-p3[0])<60 and abs(p1[1]-p0[1])>20):
            loops.append((p0,p1,p2,p3))
    chips=[]
    for m in re.finditer(r'<rect x="([-\d.]+)" y="([-\d.]+)" width="([-\d.]+)" height="20" rx="4" fill="#ffffff" stroke="#d5d9ec"/>\s*<text[^>]*>([^<]*)</text>', s):
        x,y,w,t=m.groups(); chips.append((float(x),float(y),float(w),20.0,t))
    print('==',f,'loops',len(loops),'chips',len(chips))
    for (x,y,w,h,t) in chips:
        hit=[]
        for i,l in enumerate(loops):
            n=sum(1 for k in range(201) if (lambda q: x<q[0]<x+w and y<q[1]<y+h)(bez(*l,k/200)))
            if n: hit.append((i,n))
        print('   chip %-8s x %.1f..%.1f y %.1f..%.1f  loop samples inside (loop#,of 201): %s' % (t,x,x+w,y,y+h,hit or 'none'))
    for i,l in enumerate(loops): print('   loop',i,l)
