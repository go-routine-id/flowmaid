import re,sys
def parse(d):
    toks=re.findall(r'[MLCQZ]|-?\d+\.?\d*',d)
    pts=[];i=0;cur=None
    def num(k): return [float(toks[i+1+j]) for j in range(k)]
    while i<len(toks):
        t=toks[i]
        if t=='M': x,y=num(2); cur=(x,y); pts.append(cur); i+=3
        elif t=='L': x,y=num(2); cur=(x,y); pts.append(cur); i+=3
        elif t=='C':
            a=num(6); p0=cur; p1=(a[0],a[1]); p2=(a[2],a[3]); p3=(a[4],a[5])
            for k in range(1,13):
                s=k/12; u=1-s
                pts.append((u**3*p0[0]+3*u*u*s*p1[0]+3*u*s*s*p2[0]+s**3*p3[0], u**3*p0[1]+3*u*u*s*p1[1]+3*u*s*s*p2[1]+s**3*p3[1]))
            cur=p3; i+=7
        elif t=='Q':
            a=num(4); p0=cur; p1=(a[0],a[1]); p2=(a[2],a[3])
            for k in range(1,9):
                s=k/8; u=1-s
                pts.append((u*u*p0[0]+2*u*s*p1[0]+s*s*p2[0], u*u*p0[1]+2*u*s*p1[1]+s*s*p2[1]))
            cur=p2; i+=5
        elif t=='Z': i+=1
        else:
            # implicit repeat of previous command (C continued)
            a=[float(toks[i+j]) for j in range(6)]; p0=cur; p1=(a[0],a[1]); p2=(a[2],a[3]); p3=(a[4],a[5])
            for k in range(1,13):
                s=k/12; u=1-s
                pts.append((u**3*p0[0]+3*u*u*s*p1[0]+3*u*s*s*p2[0]+s**3*p3[0], u**3*p0[1]+3*u*u*s*p1[1]+3*u*s*s*p2[1]+s**3*p3[1]))
            cur=p3; i+=6
    return pts
def cross(a,b,c,d):
    def side(o,p,q): return (p[0]-o[0])*(q[1]-o[1])-(p[1]-o[1])*(q[0]-o[0])
    return side(c,d,a)*side(c,d,b)<0 and side(a,b,c)*side(a,b,d)<0
def edges(svg):
    out=[]
    for m in re.finditer(r'<path d="([^"]*)"([^>]*)>',svg):
        d,rest=m.group(1),m.group(2)
        if 'Z' in d: break
        if 'fill="none"' in rest: out.append(parse(d))
    return out
def count(svg):
    es=edges(svg); n=0
    for i in range(len(es)):
        for j in range(i+1,len(es)):
            for k in range(len(es[i])-1):
                for l in range(len(es[j])-1):
                    if cross(es[i][k],es[i][k+1],es[j][l],es[j][l+1]): n+=1
    curves=0
    return n,len(es)
if __name__=='__main__':
    for f in sys.argv[1:]:
        s=open(f).read(); n,e=count(s); print(f.split('/')[-1],'edges',e,'crossings',n)
