import re,sys,html
s=open(sys.argv[1]).read()
paths=re.findall(r'<path d="([^"]*)"[^>]*><title>([^<]*)</title>',s)
segs=[]
for d,t in paths:
    toks=re.findall(r'[MLCAZ]|-?[\d.]+',d); i=0; cur=None; cmd=None; pts=[]
    while i<len(toks):
        if toks[i].isalpha(): cmd=toks[i]; i+=1; continue
        if cmd in 'ML': p=(float(toks[i]),float(toks[i+1])); i+=2
        elif cmd=='C': p=(float(toks[i+4]),float(toks[i+5])); i+=6
        elif cmd=='A': p=(float(toks[i+5]),float(toks[i+6])); i+=7
        else: i+=1; continue
        if cur and cmd=='L': segs.append((html.unescape(t),cur,p))
        cur=p
for a in range(len(segs)):
    for b in range(a+1,len(segs)):
        ta,p1,p2=segs[a]; tb,q1,q2=segs[b]
        if ta==tb: continue
        if abs(p1[0]-p2[0])<0.1 and abs(q1[0]-q2[0])<0.1 and abs(p1[0]-q1[0])<4:
            lo=max(min(p1[1],p2[1]),min(q1[1],q2[1])); hi=min(max(p1[1],p2[1]),max(q1[1],q2[1]))
            if hi-lo>3: print(f"V gap {abs(p1[0]-q1[0]):.2f} x={p1[0]:.1f} y {lo:.0f}-{hi:.0f} | {ta} | {tb}")
        if abs(p1[1]-p2[1])<0.1 and abs(q1[1]-q2[1])<0.1 and abs(p1[1]-q1[1])<4:
            lo=max(min(p1[0],p2[0]),min(q1[0],q2[0])); hi=min(max(p1[0],p2[0]),max(q1[0],q2[0]))
            if hi-lo>3: print(f"H gap {abs(p1[1]-q1[1]):.2f} y={p1[1]:.1f} x {lo:.0f}-{hi:.0f} | {ta} | {tb}")
