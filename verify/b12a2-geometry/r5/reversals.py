import re,sys,os
def pts(d):
    # extract the end points of each command in sequence (M/L/C endpoints), keeping the command letter
    toks=re.findall(r'([MLCA])([^MLCAZ]*)',d)
    out=[]
    for c,a in toks:
        n=[float(x) for x in re.findall(r'-?\d+\.?\d*',a)]
        if c in 'ML': out.append((c,n[0],n[1]))
        elif c=='C': out.append((c,n[4],n[5]))
        elif c=='A': out.append((c,n[5],n[6]))
    return out
def revs(d):
    p=pts(d); r=0
    for i in range(len(p)-2):
        if p[i+1][0]!='L' or p[i+2][0]!='L': continue
        ax,ay=p[i+1][1]-p[i][1],p[i+1][2]-p[i][2]
        bx,by=p[i+2][1]-p[i+1][1],p[i+2][2]-p[i+1][2]
        la=(ax*ax+ay*ay)**.5; lb=(bx*bx+by*by)**.5
        if la<0.5 or lb<0.5: continue
        cos=(ax*bx+ay*by)/(la*lb)
        if cos<-0.95: r+=1
    return r
d=sys.argv[1]
tot=0
for f in sorted(os.listdir(d)):
    if not f.endswith('.svg'): continue
    s=open(os.path.join(d,f)).read()
    n=sum(revs(m) for m in re.findall(r'<path[^>]* d="([^"]+)"',s))
    if n: print(f,n); tot+=n
print('TOTAL',tot)
