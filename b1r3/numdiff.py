import re,sys,os
a,b=sys.argv[1],sys.argv[2]
tok=re.compile(r'-?\d+\.\d+|-?\d+|[^\d\-]+|-')
worst=0;nfiles=0;bad=[]
for f in sorted(os.listdir(a)):
    if not os.path.exists(os.path.join(b,f)): bad.append(f+' missing'); continue
    x=open(os.path.join(a,f)).read(); y=open(os.path.join(b,f)).read()
    if x==y: continue
    nfiles+=1
    tx=tok.findall(x); ty=tok.findall(y)
    if len(tx)!=len(ty): bad.append(f+' structure'); continue
    m=0; cnt=0
    for p,q in zip(tx,ty):
        if p==q: continue
        try: d=abs(float(p)-float(q)); m=max(m,d); cnt+=1
        except: bad.append(f+' text '+p[:30]+'|'+q[:30]); break
    worst=max(worst,m)
    print(f"{f}: {cnt} numbers differ, max {m:.3f}")
print("files",nfiles,"worst",worst,"bad",bad[:10])
