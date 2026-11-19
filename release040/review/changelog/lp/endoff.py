import re,sys
# report self-loop ends off their box: loops are single-cubic paths whose both ends lie within 40px of the same class rect
s=open(sys.argv[1]).read()
rects=[(float(a),float(b),float(c),float(d)) for a,b,c,d in re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="4" fill="none"',s)]
out=[]
for m in re.finditer(r'<path d="M ([\d.]+) ([\d.]+) C [\d.]+ [\d.]+, [\d.]+ [\d.]+, ([\d.]+) ([\d.]+)" fill="none"',s):
    a=(float(m.group(1)),float(m.group(2))); b=(float(m.group(3)),float(m.group(4)))
    best=None
    for (x,y,w,h) in rects:
        def dist(p): 
            dx=max(x-p[0],0,p[0]-(x+w)); dy=max(y-p[1],0,p[1]-(y+h)); return (dx*dx+dy*dy)**.5
        da,db=dist(a),dist(b)
        if da<40 and db<40 and (best is None or da+db<best[0]+best[1]): best=(da,db)
    if best and abs(a[0]-b[0])<1e-6 or best and abs(a[1]-b[1])<1e-6: out.append(max(best))
print(len(out),'loops; ends off box by', sorted(set(round(o,1) for o in out)))
