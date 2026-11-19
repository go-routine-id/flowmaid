import re,sys,glob
# end glyphs: polygons, and open-arrow pairs (two consecutive 'M a L tip' paths sharing the tip)
def glyphs(s):
    out=[]
    for pts in re.findall(r'<polygon points="([^"]*)"',s):
        p=[tuple(map(float,q.split(','))) for q in pts.split()]
        out.append(p)
    segs=re.findall(r'<path d="M ([\d.-]+) ([\d.-]+) L ([\d.-]+) ([\d.-]+)" fill="none" stroke="#44507a" stroke-width="1.6"/>',s)
    segs=[tuple(map(float,x)) for x in segs]
    i=0
    while i+1<len(segs):
        a,b=segs[i],segs[i+1]
        if (a[2],a[3])==(b[2],b[3]):
            out.append([(a[0],a[1]),(a[2],a[3]),(b[0],b[1])]); i+=2
        else: i+=1
    return out
def bb(p): xs=[q[0] for q in p]; ys=[q[1] for q in p]; return (min(xs),min(ys),max(xs),max(ys))
tot=0
for d in sys.argv[1:]:
    for f in sorted(glob.glob(d+'/*.svg')):
        g=[bb(p) for p in glyphs(open(f).read())]
        bad=[]
        for i in range(len(g)):
            for j in range(i+1,len(g)):
                a,b=g[i],g[j]
                gx=max(a[0],b[0])-min(a[2],b[2]); gy=max(a[1],b[1])-min(a[3],b[3])
                gap=max(gx,gy)
                if gap<1.6: bad.append(round(gap,1))
        if bad: tot+=1; print(f.split('/')[-1], len(bad), sorted(bad)[:6])
print('files with touching glyphs:',tot)
