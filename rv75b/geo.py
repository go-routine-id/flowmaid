import re,math,sys
def tok(d):
    return re.findall(r'[MLCQAZHVmlcqazhv]|-?\d*\.?\d+(?:e-?\d+)?',d)
def sample(d,step=4):
    t=tok(d);i=0;cmd=None;pts=[];cur=(0,0)
    def num():
        nonlocal i; v=float(t[i]); i+=1; return v
    while i<len(t):
        if re.match(r'[A-Za-z]',t[i]): cmd=t[i]; i+=1
        if cmd=='M': cur=(num(),num()); pts.append(cur); cmd='L'
        elif cmd=='L':
            p=(num(),num()); pts.append(p); cur=p
        elif cmd=='C':
            p1=(num(),num());p2=(num(),num());p3=(num(),num())
            L=math.dist(cur,p1)+math.dist(p1,p2)+math.dist(p2,p3); n=max(2,int(L/step))
            for k in range(1,n+1):
                u=k/n; a=(1-u)**3; b=3*u*(1-u)**2; c=3*u*u*(1-u); e=u**3
                pts.append((a*cur[0]+b*p1[0]+c*p2[0]+e*p3[0], a*cur[1]+b*p1[1]+c*p2[1]+e*p3[1]))
            cur=p3
        elif cmd=='Q':
            p1=(num(),num());p2=(num(),num())
            n=4
            for k in range(1,n+1):
                u=k/n; pts.append(((1-u)**2*cur[0]+2*u*(1-u)*p1[0]+u*u*p2[0],(1-u)**2*cur[1]+2*u*(1-u)*p1[1]+u*u*p2[1]))
            cur=p2
        elif cmd=='A':
            rx,ry,rot,la,sw,x,y=[num() for _ in range(7)]; pts.append((x,y)); cur=(x,y)
        elif cmd in 'Zz': break
        else: i+=1
    return pts
def edges_of(path):
    s=open(path).read()
    out=[]
    if 'flowchart-link' in s:
        for m in re.finditer(r'<path d="([^"]*)"[^>]*data-id="([^"]*)"',s):
            out.append((m.group(2),sample(m.group(1))))
    else:
        s2=re.sub(r'<defs>.*?</defs>','',s,flags=re.S)
        for l in s2.split('\n'):
            if '<path' in l and 'marker-end' in l:
                t=re.search(r'<title>(.*?)</title>',l).group(1); d=re.search(r' d="([^"]*)"',l).group(1)
                out.append((t,sample(d)))
    return out
def segx(a,b,c,d):
    r=(b[0]-a[0],b[1]-a[1]); s=(d[0]-c[0],d[1]-c[1]); den=r[0]*s[1]-r[1]*s[0]
    if abs(den)<1e-9: return None
    t=((c[0]-a[0])*s[1]-(c[1]-a[1])*s[0])/den; u=((c[0]-a[0])*r[1]-(c[1]-a[1])*r[0])/den
    if 0<=t<1 and 0<=u<1: return (a[0]+t*r[0],a[1]+t*r[1])
def crossings(E):
    tot=0
    for i in range(len(E)):
        for j in range(i+1,len(E)):
            P=E[i][1];Q=E[j][1]; found=[]
            bbP=(min(p[0] for p in P),max(p[0] for p in P),min(p[1] for p in P),max(p[1] for p in P))
            for k in range(len(P)-1):
                a,b=P[k],P[k+1]
                for m in range(len(Q)-1):
                    c,d=Q[m],Q[m+1]
                    if max(c[0],d[0])<min(a[0],b[0]) or min(c[0],d[0])>max(a[0],b[0]) or max(c[1],d[1])<min(a[1],b[1]) or min(c[1],d[1])>max(a[1],b[1]): continue
                    x=segx(a,b,c,d)
                    if x and all(math.dist(x,f)>4 for f in found): found.append(x)
            # drop crossings near shared endpoints
            ends=[P[0],P[-1],Q[0],Q[-1]]
            found=[f for f in found if min(math.dist(f,e) for e in ends)>10]
            tot+=len(found)
    return tot
if __name__=='__main__':
    for p in sys.argv[1:]:
        E=edges_of(p)
        L=sum(sum(math.dist(e[1][k],e[1][k+1]) for k in range(len(e[1])-1)) for e in E)
        print(p.split('/')[-1],len(E),'edges, length %.0f'%L,'crossings',crossings(E))
